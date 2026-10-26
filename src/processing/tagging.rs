//! FLAC Vorbis comments and WAV/MP3 ID3 tags, preserving audio and existing pictures.
use super::{Tags, command, credits::split_fields, tempfile};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::Command,
};
fn flac(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("flac"))
}
fn mp3(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
}
fn mp3_id3(path: &Path) -> Result<(u64, Vec<Frame>)> {
    let mut input = fs::File::open(path)?;
    let mut header = [0; 10];
    input.read_exact(&mut header)?;
    if &header[..3] != b"ID3" {
        return Ok((0, Vec::new()));
    }
    let len = 10 + syncsafe(&header[6..])?;
    ensure!(len <= 16 * 1024 * 1024, "oversized ID3");
    ensure!(len as u64 <= input.metadata()?.len(), "truncated ID3");
    let mut bytes = vec![0; len];
    bytes[..10].copy_from_slice(&header);
    input.read_exact(&mut bytes[10..])?;
    Ok((len as u64, frames(&bytes)?))
}
pub fn read(path: &Path) -> Result<(Tags, f64)> {
    let probe: serde_json::Value = serde_json::from_str(&command(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
            ])
            .arg(path),
    )?)?;
    let duration = probe["format"]["duration"]
        .as_str()
        .context("audio duration")?
        .parse::<f64>()?;
    ensure!(
        duration.is_finite() && duration > 0.,
        "invalid audio duration"
    );
    let mut tags = Tags::new();
    if flac(path) {
        let text = command(Command::new("metaflac").arg("--export-tags-to=-").arg(path))?;
        for l in text.lines() {
            if let Some((k, v)) = l.split_once('=') {
                tags.entry(k.to_uppercase()).or_default().push(v.into());
            }
        }
    } else {
        for object in [
            probe["format"]["tags"].as_object(),
            probe["streams"]
                .as_array()
                .and_then(|s| s.first())
                .and_then(|s| s["tags"].as_object()),
        ]
        .into_iter()
        .flatten()
        {
            for (k, v) in object {
                if let Some(s) = v.as_str() {
                    tags.entry(k.to_uppercase())
                        .or_insert_with(|| vec![s.into()]);
                }
            }
        }
        for data in if mp3(path)
            || path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
        {
            audio_id3(path)?
        } else {
            Vec::new()
        } {
            let (name, values) = decode_text(&data.0, &data.1)?;
            if let Some(name) = name {
                tags.insert(name, values);
            }
        }
    }
    for (from, to) in [
        ("ALBUM_ARTIST", "ALBUMARTIST"),
        ("TRACK", "TRACKNUMBER"),
        ("DISC", "DISCNUMBER"),
        ("CATALOGID", "CATALOGNUMBER"),
        ("COUNTRY", "RELEASECOUNTRY"),
        ("YEAR", "DATE"),
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
            .flat_map(|value| value.split(';'))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect();
    }
    split_fields(&mut tags);
    Ok((tags, duration))
}
fn syncsafe(bytes: &[u8]) -> Result<usize> {
    ensure!(
        bytes.len() == 4 && bytes.iter().all(|b| b & 128 == 0),
        "invalid ID3 size"
    );
    Ok(bytes.iter().fold(0, |n, b| (n << 7) | *b as usize))
}
fn size(n: usize) -> [u8; 4] {
    [
        ((n >> 21) & 127) as u8,
        ((n >> 14) & 127) as u8,
        ((n >> 7) & 127) as u8,
        (n & 127) as u8,
    ]
}
type Frame = (String, Vec<u8>);
fn frames(bytes: &[u8]) -> Result<Vec<Frame>> {
    ensure!(bytes.len() >= 10 && &bytes[..3] == b"ID3", "invalid ID3");
    let version = bytes[3];
    ensure!(version == 3 || version == 4, "unsupported ID3 version");
    ensure!(bytes[5] == 0, "flagged ID3 requires review");
    let end = 10 + syncsafe(&bytes[6..10])?;
    ensure!(end <= bytes.len(), "truncated ID3");
    let mut pos = 10;
    let mut out = Vec::new();
    while pos + 10 <= end && bytes[pos] != 0 {
        let id = std::str::from_utf8(&bytes[pos..pos + 4])?.to_string();
        let n = if version == 4 {
            syncsafe(&bytes[pos + 4..pos + 8])?
        } else {
            u32::from_be_bytes(bytes[pos + 4..pos + 8].try_into()?) as usize
        };
        ensure!(
            bytes[pos + 8..pos + 10] == [0, 0],
            "flagged ID3 frame requires review"
        );
        pos += 10;
        ensure!(pos + n <= end, "truncated ID3 frame");
        out.push((id, bytes[pos..pos + n].to_vec()));
        pos += n;
    }
    Ok(out)
}
fn text(data: &[u8]) -> Result<String> {
    if data.is_empty() {
        return Ok(String::new());
    }
    match data[0] {
        0 => Ok(data[1..].iter().map(|b| char::from(*b)).collect()),
        3 => Ok(String::from_utf8(data[1..].to_vec())?),
        1 | 2 => {
            let b = &data[1..];
            let (little, b) = if b.starts_with(&[255, 254]) {
                (true, &b[2..])
            } else if b.starts_with(&[254, 255]) {
                (false, &b[2..])
            } else {
                (false, b)
            };
            ensure!(b.len() % 2 == 0, "invalid UTF16 tag");
            let words = b
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| {
                    if little {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect::<Vec<_>>();
            Ok(String::from_utf16(&words)?)
        }
        _ => bail!("unsupported ID3 text encoding"),
    }
}
fn standard(id: &str) -> Option<&'static str> {
    Some(match id {
        "TALB" => "ALBUM",
        "TPE1" => "ARTIST",
        "TPE2" => "ALBUMARTIST",
        "TSOP" => "ARTISTSORT",
        "TSO2" => "ALBUMARTISTSORT",
        "TIT2" => "TITLE",
        "TRCK" => "TRACKNUMBER",
        "TPOS" => "DISCNUMBER",
        "TDRC" | "TYER" => "DATE",
        "TCON" => "GENRE",
        "TCOM" => "COMPOSER",
        "TPE3" => "CONDUCTOR",
        "TPE4" => "REMIXER",
        "TPUB" => "PUBLISHER",
        _ => return None,
    })
}
fn decode_text(id: &str, data: &[u8]) -> Result<(Option<String>, Vec<String>)> {
    if id == "TXXX" {
        let s = text(data)?;
        let (k, v) = s.split_once('\0').context("invalid TXXX")?;
        return Ok((
            Some(k.to_uppercase()),
            v.split('\0')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        ));
    }
    if let Some(k) = standard(id) {
        return Ok((
            Some(k.into()),
            text(data)?
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        ));
    }
    Ok((None, vec![]))
}
fn chunks(path: &Path) -> Result<Vec<([u8; 4], u64, u64)>> {
    let mut f = fs::File::open(path)?;
    let len = f.metadata()?.len();
    let mut header = [0; 12];
    f.read_exact(&mut header)?;
    ensure!(
        &header[..4] == b"RIFF" && &header[8..] == b"WAVE",
        "only RIFF WAV supported"
    );
    let declared = u32::from_le_bytes(header[4..8].try_into()?) as u64 + 8;
    // Accept the observed +4 header error only when every chunk fits the actual file below.
    ensure!(
        declared == len || declared == len + 4,
        "WAV length mismatch"
    );
    let mut out = Vec::new();
    let mut pos = 12;
    while pos + 8 <= len {
        f.seek(SeekFrom::Start(pos))?;
        let mut h = [0; 8];
        f.read_exact(&mut h)?;
        let n = u32::from_le_bytes(h[4..].try_into()?) as u64;
        ensure!(pos + 8 + n + (n % 2) <= len, "truncated WAV chunk");
        out.push((h[..4].try_into()?, pos + 8, n));
        pos += 8 + n + n % 2;
    }
    ensure!(pos == len, "trailing WAV bytes");
    if declared != len {
        eprintln!("WAV RIFF length is four bytes too large; all chunks are complete");
    }
    Ok(out)
}
fn audio_id3(path: &Path) -> Result<Vec<Frame>> {
    if mp3(path) {
        return Ok(mp3_id3(path)?.1);
    }
    let mut out = Vec::new();
    let mut f = fs::File::open(path)?;
    for (id, pos, n) in chunks(path)? {
        if id.eq_ignore_ascii_case(b"id3 ") {
            ensure!(n <= 16 * 1024 * 1024, "oversized ID3");
            f.seek(SeekFrom::Start(pos))?;
            let mut b = vec![0; n as usize];
            f.read_exact(&mut b)?;
            out.extend(frames(&b)?);
        }
    }
    Ok(out)
}
pub fn write(path: &Path, tags: &Tags) -> Result<()> {
    if !flac(path)
        && !mp3(path)
        && !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
    {
        return super::write_container_tags(path, tags);
    }
    if flac(path) {
        let mut cmd = Command::new("metaflac");
        for k in tags.keys() {
            ensure!(!k.contains(['\n', '\r', '=']), "invalid tag name");
            cmd.arg(format!("--remove-tag={k}"));
        }
        cmd.arg(path);
        command(&mut cmd)?;
        let mut input = Vec::new();
        for (key, values) in tags {
            for value in values {
                ensure!(
                    !value.contains(['\n', '\r', '\0']),
                    "multiline tag requires review"
                );
                writeln!(input, "{key}={value}")?;
            }
        }
        let temp = tempfile::tempdir_in(path.parent().context("audio parent")?)?;
        let import = temp.path().join("tags");
        fs::write(&import, input)?;
        command(
            Command::new("metaflac")
                .arg(format!("--import-tags-from={}", import.display()))
                .arg(path),
        )?;
        return Ok(());
    }
    let mut saved = Vec::new();
    for frame in audio_id3(path)? {
        let (key, _) = decode_text(&frame.0, &frame.1)?;
        if key
            .as_deref()
            .is_some_and(|k| matches!(k, "CATALOGNUMBER" | "CATALOGID") && !tags.contains_key(k))
        {
            continue;
        }
        if key.is_none_or(|k| !tags.contains_key(&k)) {
            saved.push(frame);
        }
    }
    for (k, values) in tags {
        if let Some((id, _)) = [
            ("TALB", "ALBUM"),
            ("TPE1", "ARTIST"),
            ("TPE2", "ALBUMARTIST"),
            ("TSOP", "ARTISTSORT"),
            ("TSO2", "ALBUMARTISTSORT"),
            ("TIT2", "TITLE"),
            ("TRCK", "TRACKNUMBER"),
            ("TPOS", "DISCNUMBER"),
            ("TDRC", "DATE"),
            ("TCON", "GENRE"),
            ("TCOM", "COMPOSER"),
            ("TPE3", "CONDUCTOR"),
            ("TPE4", "REMIXER"),
            ("TPUB", "PUBLISHER"),
        ]
        .iter()
        .find(|(_, key)| *key == k)
        {
            let mut b = vec![3];
            b.extend(values.join("\0").as_bytes());
            saved.push((id.to_string(), b));
        }
        let mut b = vec![3];
        b.extend(k.as_bytes());
        b.push(0);
        b.extend(values.join("\0").as_bytes());
        saved.push(("TXXX".into(), b));
    }
    let mut payload = Vec::new();
    for (id, data) in saved {
        payload.extend(id.as_bytes());
        payload.extend(size(data.len()));
        payload.extend([0, 0]);
        payload.extend(data);
    }
    let mut id3 = b"ID3\x04\x00\x00".to_vec();
    id3.extend(size(payload.len()));
    id3.extend(payload);
    let temp = tempfile::tempdir_in(path.parent().context("audio parent")?)?;
    if mp3(path) {
        let offset = mp3_id3(path)?.0;
        let dest = temp.path().join("tagged.mp3");
        let mut output = fs::File::create_new(&dest)?;
        output.write_all(&id3)?;
        let mut input = fs::File::open(path)?;
        input.seek(SeekFrom::Start(offset))?;
        let copied = std::io::copy(&mut input, &mut output)?;
        ensure!(
            copied == input.metadata()?.len() - offset,
            "MP3 copy truncated"
        );
        output.sync_all()?;
        drop(output);
        fs::rename(dest, path)?;
        return Ok(());
    }
    let dest = temp.path().join("tagged.wav");
    let mut output = fs::File::create_new(&dest)?;
    let mut input = fs::File::open(path)?;
    output.write_all(b"RIFF\0\0\0\0WAVE")?;
    for (id, pos, n) in chunks(path)? {
        if id.eq_ignore_ascii_case(b"id3 ") {
            continue;
        }
        output.write_all(&id)?;
        output.write_all(&(n as u32).to_le_bytes())?;
        input.seek(SeekFrom::Start(pos))?;
        let copied = std::io::copy(&mut Read::by_ref(&mut input).take(n + n % 2), &mut output)?;
        ensure!(copied == n + n % 2, "WAV copy truncated");
    }
    output.write_all(b"id3 ")?;
    output.write_all(&(id3.len() as u32).to_le_bytes())?;
    output.write_all(&id3)?;
    if !id3.len().is_multiple_of(2) {
        output.write_all(&[0])?;
    }
    let len = output.stream_position()?;
    ensure!(len - 8 <= u32::MAX as u64, "WAV exceeds RIFF limit");
    output.seek(SeekFrom::Start(4))?;
    output.write_all(&((len - 8) as u32).to_le_bytes())?;
    output.sync_all()?;
    drop(output);
    fs::rename(dest, path)?;
    Ok(())
}

pub fn picture(path: &Path) -> Result<Option<Vec<u8>>> {
    // ffmpeg reads FLAC pictures and WAV/MP3 APIC without changing the source.
    let probe: serde_json::Value = serde_json::from_str(&command(
        Command::new("ffprobe")
            .args(["-v", "error", "-show_streams", "-of", "json"])
            .arg(path),
    )?)?;
    if !probe["streams"].as_array().is_some_and(|s| {
        s.iter()
            .any(|s| s["disposition"]["attached_pic"].as_u64() == Some(1))
    }) {
        return Ok(None);
    }
    let out = super::command_output(
        Command::new("ffmpeg")
            .args(["-v", "error", "-nostdin", "-i"])
            .arg(path)
            .args([
                "-map",
                "0:v:0",
                "-frames:v",
                "1",
                "-f",
                "image2pipe",
                "-c:v",
                "mjpeg",
                "-",
            ]),
    )?;
    Ok(Some(out))
}
