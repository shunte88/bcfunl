//! Native metadata handling follows VCW export's Vorbis, ID3 and MP4 backends.
use super::{Tags, credits::split_fields};
use anyhow::{Result, bail, ensure};
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::{AudioFile, TaggedFileExt},
    id3::v2::{Frame, Id3v2Tag},
    mp4::{Atom, AtomData, AtomIdent, Ilst},
    ogg::{OggPictureStorage, tag::VorbisComments},
    prelude::{ItemKey, TagExt},
    tag::{ItemValue, Tag, TagItem, TagType},
};
use std::{fs::File, path::Path};

enum Native {
    Vorbis(VorbisComments),
    Id3(Id3v2Tag),
    Mp4(Ilst),
}
fn native(path: &Path) -> Result<Native> {
    let mut file = File::open(path)?;
    let options = ParseOptions::new();
    Ok(match super::extension(path).as_str() {
        "flac" => {
            let file = lofty::flac::FlacFile::read_from(&mut file, options)?;
            let mut comments = file.vorbis_comments().cloned().unwrap_or_default();
            for (picture, info) in file.pictures() {
                comments.insert_picture(picture.clone(), Some(*info))?;
            }
            Native::Vorbis(comments)
        }
        "ogg" => Native::Vorbis(
            lofty::ogg::VorbisFile::read_from(&mut file, options)?
                .vorbis_comments()
                .clone(),
        ),
        "mp3" => Native::Id3(
            lofty::mpeg::MpegFile::read_from(&mut file, options)?
                .id3v2()
                .cloned()
                .unwrap_or_default(),
        ),
        "wav" => Native::Id3(
            lofty::iff::wav::WavFile::read_from(&mut file, options)?
                .id3v2()
                .cloned()
                .unwrap_or_default(),
        ),
        "aif" | "aiff" | "aifc" => Native::Id3(
            lofty::iff::aiff::AiffFile::read_from(&mut file, options)?
                .id3v2()
                .cloned()
                .unwrap_or_default(),
        ),
        "m4a" => Native::Mp4(
            lofty::mp4::Mp4File::read_from(&mut file, options)?
                .ilst()
                .cloned()
                .unwrap_or_default(),
        ),
        _ => bail!("Unsupported tag container: {}", path.display()),
    })
}
fn add(tags: &mut Tags, key: &str, value: &str) {
    for value in value.split('\0').filter(|s| !s.is_empty()) {
        let values = tags.entry(key.to_uppercase()).or_default();
        if !values.iter().any(|v| v == value) {
            values.push(value.into());
        }
    }
}
fn generic(tag: &Tag) -> Tags {
    let mut tags = Tags::new();
    for item in tag.items() {
        if let Some(key) = item.key().map_key(TagType::VorbisComments) {
            match item.value() {
                ItemValue::Text(text) | ItemValue::Locator(text) => add(&mut tags, key, text),
                _ => (),
            }
        }
    }
    tags
}
pub(super) fn read_tags(path: &Path) -> Result<Tags> {
    let native = native(path)?;
    let mut tags = match &native {
        Native::Vorbis(comments) => {
            let mut tags = Tags::new();
            for (key, value) in comments.items() {
                add(&mut tags, key, value);
            }
            tags
        }
        Native::Id3(id3) => generic(&Tag::from(id3.clone())),
        Native::Mp4(ilst) => generic(&Tag::from(ilst.clone())),
    };
    match native {
        Native::Id3(id3) => {
            for frame in &id3 {
                if let Frame::UserText(text) = frame {
                    // Canonical freeform values take precedence over lossy generic mappings.
                    tags.insert(
                        text.description.to_uppercase(),
                        text.content
                            .split('\0')
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect(),
                    );
                }
            }
        }
        Native::Mp4(ilst) => {
            for atom in &ilst {
                if let AtomIdent::Freeform { name, .. } = atom.ident() {
                    let values: Vec<String> = atom
                        .data()
                        .filter_map(|data| match data {
                            AtomData::UTF8(s) | AtomData::UTF16(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    if !values.is_empty() {
                        tags.insert(name.to_uppercase(), values);
                    }
                }
            }
        }
        _ => (),
    }
    Ok(normalize_tags(tags))
}
pub fn read(path: &Path) -> Result<(Tags, f64)> {
    crate::shutdown::check()?;
    let file = lofty::read_from_path(path)?;
    let duration = file.properties().duration().as_secs_f64();
    ensure!(
        duration.is_finite() && duration > 0.,
        "Invalid audio duration: {}",
        path.display()
    );
    let mut tags = Tags::new();
    for tag in file.tags() {
        tags.extend(generic(tag));
    }
    tags.extend(read_tags(path)?);
    Ok((normalize_tags(tags), duration))
}
fn normalize_tags(mut tags: Tags) -> Tags {
    for (from, to) in [
        ("ALBUM_ARTIST", "ALBUMARTIST"),
        ("TRACK", "TRACKNUMBER"),
        ("DISC", "DISCNUMBER"),
        ("CATALOGID", "CATALOGNUMBER"),
        ("COUNTRY", "RELEASECOUNTRY"),
        ("YEAR", "DATE"),
        ("TRACKTOTAL", "TOTALTRACKS"),
        ("DISCTOTAL", "TOTALDISCS"),
    ] {
        if super::value(&tags, to).is_empty()
            && let Some(v) = tags.get(from).cloned()
        {
            tags.insert(to.into(), v);
        }
    }
    if let Some(values) = tags.get_mut("GENRE") {
        *values = values
            .iter()
            .flat_map(|v| v.split(';'))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect();
    }
    split_fields(&mut tags);
    tags
}
fn mapped(tags: &Tags, kind: TagType) -> Tag {
    let mut tag = Tag::new(kind);
    for (key, values) in tags {
        if let Some(item) = ItemKey::from_key(TagType::VorbisComments, key) {
            for value in values {
                tag.push(TagItem::new(item, ItemValue::Text(value.clone())));
            }
        }
    }
    tag
}
fn replaced(tags: &Tags, key: &str) -> bool {
    tags.keys().any(|k| k.eq_ignore_ascii_case(key))
        || ["CATALOGNUMBER", "CATALOGID", "GENRE"]
            .iter()
            .any(|k| k.eq_ignore_ascii_case(key))
}
pub fn write(path: &Path, tags: &Tags) -> Result<()> {
    crate::shutdown::check()?;
    let tags = normalize_tags(tags.clone());
    let tags = &tags;
    for (key, values) in tags {
        ensure!(
            !key.is_empty() && !key.contains(['=', '\0', '\r', '\n']),
            "Invalid tag name"
        );
        ensure!(
            values.iter().all(|v| !v.contains('\0')),
            "NUL in tag value requires review: {key}"
        );
    }
    match native(path)? {
        Native::Vorbis(mut comments) => {
            for key in ["CATALOGNUMBER", "CATALOGID", "GENRE"] {
                drop(comments.remove(key));
            }
            for (key, values) in tags {
                drop(comments.remove(key));
                for value in values {
                    comments.push(key.clone(), value.clone());
                }
            }
            comments.save_to_path(path, WriteOptions::default())?;
        }
        Native::Id3(mut id3) => {
            // VCW writes standard frames plus user text for arbitrary metadata.
            id3.retain(|frame| !matches!(frame, Frame::UserText(text) if replaced(tags, &text.description)));
            for key in ["CATALOGNUMBER", "CATALOGID", "GENRE"] {
                id3.remove_user_text(key);
                if let Some(id) = ItemKey::from_key(TagType::VorbisComments, key)
                    .and_then(|k| k.map_key(TagType::Id3v2))
                    && id.len() == 4
                {
                    drop(id3.remove(&lofty::id3::v2::FrameId::new(id)?));
                }
            }
            let mapped = Id3v2Tag::from(mapped(tags, TagType::Id3v2));
            for frame in mapped {
                id3.insert(frame);
            }
            for (key, values) in tags {
                id3.insert_user_text(key.clone(), values.join("\0"));
            }
            id3.save_to_path(path, WriteOptions::default())?;
        }
        Native::Mp4(mut ilst) => {
            ilst.retain(|atom| !matches!(atom.ident(), AtomIdent::Freeform { name, .. } if replaced(tags, name)));
            for key in ["CATALOGNUMBER", "CATALOGID", "GENRE"] {
                let ident = AtomIdent::Freeform {
                    mean: "com.apple.iTunes".into(),
                    name: key.into(),
                };
                drop(ilst.remove(&ident));
            }
            drop(ilst.remove(&AtomIdent::Fourcc(*b"\xa9gen")));
            drop(ilst.remove(&AtomIdent::Fourcc(*b"gnre")));
            let mut mapped = Ilst::from(mapped(tags, TagType::Mp4Ilst));
            for fourcc in [*b"\xa9ART", *b"aART", *b"\xa9gen"] {
                let ident = AtomIdent::Fourcc(fourcc);
                let data: Vec<_> = mapped.remove(&ident).flat_map(Atom::into_data).collect();
                if let Some(atom) = Atom::from_collection(ident, data) {
                    mapped.insert(atom);
                }
            }
            for atom in mapped {
                ilst.replace_atom(atom);
            }
            for (key, values) in tags {
                let ident = AtomIdent::Freeform {
                    mean: "com.apple.iTunes".into(),
                    name: key.clone().into(),
                };
                drop(ilst.remove(&ident));
                if let Some(atom) = Atom::from_collection(
                    ident,
                    values.iter().cloned().map(AtomData::UTF8).collect(),
                ) {
                    ilst.insert(atom);
                }
            }
            // One standard atom with multiple data boxes, as in VCW's MP4 writer.
            for fourcc in [*b"\xa9ART", *b"aART", *b"\xa9gen"] {
                let ident = AtomIdent::Fourcc(fourcc);
                let data: Vec<_> = ilst.remove(&ident).flat_map(Atom::into_data).collect();
                if let Some(atom) = Atom::from_collection(ident, data) {
                    ilst.insert(atom);
                }
            }
            ilst.save_to_path(path, WriteOptions::default())?;
        }
    }
    // Keep alternate stores (RIFF INFO, AIFF text, ID3v1/APE) consistent too.
    let file = lofty::read_from_path(path)?;
    for mut tag in file
        .tags()
        .iter()
        .filter(|tag| {
            matches!(
                tag.tag_type(),
                TagType::RiffInfo | TagType::AiffText | TagType::Id3v1 | TagType::Ape
            )
        })
        .cloned()
    {
        for key in ["CATALOGNUMBER", "CATALOGID", "GENRE"] {
            if let Some(key) = ItemKey::from_key(TagType::VorbisComments, key) {
                tag.remove_key(key);
            }
        }
        for (key, values) in tags {
            if let Some(key) = ItemKey::from_key(TagType::VorbisComments, key) {
                tag.remove_key(key);
                for value in values {
                    tag.push(TagItem::new(key, ItemValue::Text(value.clone())));
                }
            }
        }
        tag.save_to_path(path, WriteOptions::default())?;
    }
    Ok(())
}
pub fn picture(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = lofty::read_from_path(path)?;
    Ok(file
        .tags()
        .iter()
        .flat_map(|t| t.pictures())
        .next()
        .map(|p| p.data().to_vec()))
}
