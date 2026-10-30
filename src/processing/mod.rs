//! Offline processing of owned downloads. Credit and native tag handling come from mcfunl.
pub(crate) mod clean;
mod credits;
mod genres;
mod tagging;
pub(crate) mod tempfile;
mod template;

use crate::{Args, aria2::ChildGuard, db::Database, shutdown};
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
type Tags = BTreeMap<String, Vec<String>>;

fn command(cmd: &mut Command) -> Result<String> {
    Ok(String::from_utf8(command_output(cmd)?)?)
}
fn command_output(cmd: &mut Command) -> Result<Vec<u8>> {
    shutdown::check()?;
    let program = cmd.get_program().to_string_lossy().into_owned();
    let mut child = ChildGuard(
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Start {program}"))?,
    );
    let mut stdout = child.0.stdout.take().context("Command stdout")?;
    let mut stderr = child.0.stderr.take().context("Command stderr")?;
    thread::scope(|scope| {
        let out = scope.spawn(move || {
            let mut data = Vec::new();
            stdout.read_to_end(&mut data).map(|_| data)
        });
        let err = scope.spawn(move || {
            let mut data = Vec::new();
            stderr.read_to_end(&mut data).map(|_| data)
        });
        let status = child.wait();
        if status.is_err() {
            let _ = child.0.kill();
            let _ = child.0.wait();
        }
        let out = out
            .join()
            .map_err(|_| anyhow::anyhow!("Command output reader panicked"))??;
        let err = err
            .join()
            .map_err(|_| anyhow::anyhow!("Command error reader panicked"))??;
        ensure!(
            status?.success(),
            "{program} failed: {}",
            String::from_utf8_lossy(&err).trim()
        );
        Ok(out)
    })
}
fn value(tags: &Tags, key: &str) -> String {
    tags.get(key)
        .and_then(|v| v.first())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty() && !(key == "CATALOGNUMBER" && v.eq_ignore_ascii_case("none")))
        .unwrap_or_default()
        .into()
}
fn put(tags: &mut Tags, key: &str, val: impl ToString) {
    let val = val.to_string();
    if !val.trim().is_empty() {
        tags.insert(key.into(), vec![val]);
    }
}
fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
fn audio(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "flac" | "wav" | "mp3" | "aiff" | "aif" | "aifc" | "m4a" | "ogg"
    )
}
fn collect(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in
        fs::read_dir(root).with_context(|| format!("Read release directory {}", root.display()))?
    {
        shutdown::check()?;
        let entry = entry?;
        let kind = entry
            .file_type()
            .with_context(|| format!("Inspect release member {}", entry.path().display()))?;
        ensure!(
            !kind.is_symlink(),
            "Symlink refused in release: {}",
            entry.path().display()
        );
        if kind.is_dir() {
            files.extend(collect(&entry.path())?);
        } else {
            ensure!(kind.is_file(), "Nonregular release member refused");
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}
fn validate_listing(listing: &str) -> Result<()> {
    let mut paths = HashSet::new();
    for line in listing.lines() {
        if let Some(path) = line.strip_prefix("Path = ") {
            ensure!(
                !path.is_empty()
                    && !path.contains(['\\', '\0'])
                    && !Path::new(path).is_absolute()
                    && !path
                        .split('/')
                        .any(|part| part == ".." || part.contains(':')),
                "Unsafe archive member"
            );
            ensure!(paths.insert(path), "Duplicate archive member");
        }
        ensure!(
            !line.split_once(" = ").is_some_and(|(key, val)| matches!(
                key,
                "Symbolic Link" | "Hard Link" | "Copy Link"
            ) && !val.trim().is_empty())
                && !line
                    .strip_prefix("Attributes = ")
                    .is_some_and(|v| v.split_whitespace().any(|a| a.starts_with('l'))),
            "Archive link refused"
        );
    }
    Ok(())
}
fn extract(source: &Path, destination: &Path) -> Result<Vec<PathBuf>> {
    let listing = command(
        Command::new("7z")
            .args(["l", "-slt", "-ba", "--"])
            .arg(source),
    )?;
    validate_listing(&listing)?;
    command(
        Command::new("7z")
            .args(["x", "-y", "-p", "-snl-"])
            .arg(format!("-o{}", destination.display()))
            .arg("--")
            .arg(source),
    )?;
    collect(destination)
}
fn retry_audio<T>(path: &Path, mut read: impl FnMut() -> Result<T>) -> Result<T> {
    match read() {
        Ok(value) => Ok(value),
        Err(error) if shutdown::requested() => Err(error),
        Err(error) => {
            eprintln!(
                "  Audio read failed for {}; retrying once: {error:#}",
                path.display()
            );
            shutdown::pause(std::time::Duration::from_millis(200))?;
            read().with_context(|| {
                format!(
                    "Audio validation failed after 2 attempts: {}",
                    path.display()
                )
            })
        }
    }
}
fn pcm_hash(path: &Path) -> Result<String> {
    retry_audio(path, || pcm_hash_once(path))
}
fn measured<T>(
    duration: &mut std::time::Duration,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let started = std::time::Instant::now();
    let result = operation();
    *duration += started.elapsed();
    result
}
fn pcm_hash_once(path: &Path) -> Result<String> {
    let layout = command(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=sample_rate,channels",
                "-of",
                "csv=p=0",
            ])
            .arg(path),
    )
    .with_context(|| format!("Inspect audio layout {}", path.display()))?;
    ensure!(!layout.trim().is_empty(), "Missing audio stream layout");
    let hash = command(
        Command::new("ffmpeg")
            .args(["-v", "error", "-xerror", "-nostdin", "-i"])
            .arg(path)
            .args([
                "-map",
                "0:a:0",
                "-c:a",
                "pcm_f64le",
                "-f",
                "hash",
                "-hash",
                "sha256",
                "-",
            ]),
    )
    .with_context(|| format!("Decode audio {}", path.display()))?;
    Ok(format!("{}:{}", layout.trim(), hash.trim()))
}

fn write_container_tags(path: &Path, tags: &Tags) -> Result<()> {
    let temp = tempfile::tempdir_in(path.parent().context("Audio parent")?)?;
    let output = temp.path().join(format!("tagged.{}", extension(path)));
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-xerror", "-nostdin", "-n", "-i"])
        .arg(path)
        .args(["-map", "0", "-c", "copy", "-map_metadata", "0"]);
    for (key, vals) in tags {
        ensure!(!key.contains(['=', '\0', '\r', '\n']), "Invalid tag key");
        let separator = if key == "GENRE" { ";" } else { " / " };
        cmd.arg("-metadata")
            .arg(format!("{key}={}", vals.join(separator)));
    }
    match extension(path).as_str() {
        "m4a" => {
            cmd.args(["-movflags", "use_metadata_tags"]);
        }
        "aif" | "aiff" | "aifc" => {
            cmd.args(["-write_id3v2", "1"]);
        }
        _ => {}
    }
    command(cmd.arg(&output))?;
    fs::rename(output, path)?;
    Ok(())
}
#[derive(Clone)]
struct Track {
    path: PathBuf,
    tags: Tags,
    duration: f64,
}
fn tracks(paths: &[PathBuf]) -> Result<Vec<Track>> {
    let mut result = Vec::new();
    for path in paths.iter().filter(|p| audio(p)) {
        let (tags, duration) =
            tagging::read(path).with_context(|| format!("Read {}", path.display()))?;
        result.push(Track {
            path: path.clone(),
            tags,
            duration,
        });
    }
    ensure!(!result.is_empty(), "No supported audio found");
    Ok(result)
}
fn common(tracks: &[Track], key: &str) -> Result<String> {
    let found: HashSet<_> = tracks
        .iter()
        .map(|t| value(&t.tags, key))
        .filter(|v| !v.is_empty())
        .collect();
    ensure!(found.len() <= 1, "Conflicting {key} values require review");
    Ok(found.into_iter().next().unwrap_or_default())
}
fn number(text: &str) -> Option<u32> {
    text.split('/')
        .next()?
        .trim()
        .parse()
        .ok()
        .filter(|n| *n > 0)
}
fn track_number(track: &Track) -> Result<u32> {
    number(&value(&track.tags, "TRACKNUMBER"))
        .or_else(|| {
            let name = track.path.file_stem()?.to_str()?;
            let prefix: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
            number(&prefix)
        })
        .context("Missing track number; tag or filename requires review")
}
fn disc_number(track: &Track) -> Result<u32> {
    let text = value(&track.tags, "DISCNUMBER");
    if text.is_empty() {
        Ok(1)
    } else {
        number(&text).context("Invalid disc number")
    }
}
fn key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
fn base_album(text: &str) -> &str {
    let mut text = text.trim();
    while let Some((base, suffix)) = text.strip_suffix(']').and_then(|s| s.rsplit_once(" [")) {
        if suffix.is_empty() {
            break;
        }
        text = base.trim();
    }
    text
}
#[derive(Clone)]
struct Release {
    key: String,
    artist: String,
    album: String,
    source: PathBuf,
    genre: String,
    country: String,
    catalog: String,
    publish: String,
    url: String,
}
struct Plan {
    artist: String,
    album: String,
    country: String,
    catalog: String,
    directory: PathBuf,
    files: Vec<PathBuf>,
    genre_fallback: Option<serde_json::Value>,
}
fn prepare_tags(
    release: &Release,
    tracks: &mut [Track],
    template: &template::Template,
) -> Result<Plan> {
    if let [track] = tracks
        && value(&track.tags, "TRACKNUMBER").is_empty()
        && track_number(track).is_err()
    {
        put(&mut track.tags, "TRACKNUMBER", 1);
    }
    for track in tracks.iter_mut() {
        credits::normalize_artists(&mut track.tags);
    }
    let mut artist = common(tracks, "ALBUMARTIST")?;
    let storefront = if artist.is_empty() {
        &release.artist
    } else {
        &artist
    };
    let performers: HashSet<_> = tracks
        .iter()
        .map(|track| key(&value(&track.tags, "ARTIST")))
        .filter(|name| !name.is_empty())
        .collect();
    // Reuse mcfunl's label-compilation evidence: distinct performers, no performing storefront, and matching title prefixes.
    let label_compilation = performers.len() > 1
        && !performers.contains(&key(storefront))
        && tracks.iter().all(|track| {
            let artist = value(&track.tags, "ARTIST");
            !artist.is_empty() && value(&track.tags, "TITLE").starts_with(&format!("{artist} - "))
        });
    let label = storefront.clone();
    if label_compilation {
        artist = "Various Artists".into();
    }
    if artist.is_empty() {
        let primaries: HashSet<_> = tracks
            .iter()
            .map(|t| value(&t.tags, "ARTIST"))
            .filter(|v| !v.is_empty())
            .collect();
        artist = match primaries.len() {
            0 => release.artist.clone(),
            1 => primaries.into_iter().next().unwrap(),
            _ => "Various Artists".into(),
        };
    }
    artist = crate::sanitizing::mapped_artists(&artist)
        .map(|names| names[0].clone())
        .unwrap_or_else(|| crate::initcap(&artist));
    ensure!(!artist.is_empty(), "Missing album artist; requires review");
    let released_single = tracks.len() == 1
        && chrono::NaiveDate::parse_from_str(&release.publish, "%Y-%m-%d")
            .is_ok_and(|date| date < chrono::Utc::now().date_naive());
    if released_single {
        for tag in ["ALBUM", "TITLE"] {
            if let Some(values) = tracks[0].tags.get_mut(tag) {
                for value in values {
                    *value = crate::sanitizing::remove_prerelease(value);
                }
            }
        }
    }
    let source_album = common(tracks, "ALBUM")?;
    let mut album = if source_album.is_empty() {
        if released_single {
            crate::sanitizing::remove_prerelease(&release.album)
        } else {
            release.album.clone()
        }
    } else {
        source_album
    };
    let source_country = common(tracks, "RELEASECOUNTRY")?;
    let country_text = if source_country.is_empty() {
        &release.country
    } else {
        &source_country
    };
    let country = if country_text.eq_ignore_ascii_case("EU") {
        "EU".into()
    } else {
        crate::metadata::country(country_text)
            .unwrap_or_default()
            .to_owned()
    };
    let source_catalog = common(tracks, "CATALOGNUMBER")?;
    let catalog = if source_catalog.is_empty() {
        release.catalog.trim().to_owned()
    } else {
        source_catalog
    };
    let catalog = if catalog.eq_ignore_ascii_case("none") {
        String::new()
    } else {
        catalog
    };
    for suffix in [format!(" [{catalog}]"), format!(" [{country}]")] {
        if let Some(base) = album.strip_suffix(&suffix) {
            album = base.into();
        }
    }
    let (clean_album, mut album_guests) = credits::album_credit(&album, &artist);
    let (captured_album, captured_guests) = credits::album_credit(&release.album, &artist);
    if key(&captured_album) == key(&clean_album) {
        for guest in captured_guests {
            credits::append_names(&mut album_guests, &guest);
        }
    }
    let (cleaned, extracted_catalog) = crate::sanitizing::title(&album, &artist);
    let catalog = if catalog.is_empty() {
        extracted_catalog.unwrap_or_default()
    } else {
        catalog
    };
    album = crate::sanitizing::title_display(&credits::album_credit(&cleaned, &artist).0);
    ensure!(!album.is_empty(), "Missing album title; requires review");
    let decorated = format!(
        "{album}{}{}",
        if country.is_empty() {
            String::new()
        } else {
            format!(" [{country}]")
        },
        if catalog.is_empty() {
            String::new()
        } else {
            format!(" [{catalog}]")
        }
    );
    let mut directory = None;
    let mut files = Vec::new();
    let mut numbers = HashSet::new();
    let mut paths = HashSet::new();
    let discs = tracks
        .iter()
        .map(disc_number)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(1);
    let mut totals = BTreeMap::<u32, usize>::new();
    for track in tracks.iter() {
        *totals.entry(disc_number(track)?).or_default() += 1;
    }
    for track in tracks {
        let disc = disc_number(track)?;
        let number = track_number(track)?;
        ensure!(
            numbers.insert((disc, number)),
            "Duplicate disc/track number"
        );
        let raw_title = value(&track.tags, "TITLE");
        let performer = value(&track.tags, "ARTIST");
        let title = if label_compilation {
            raw_title
                .strip_prefix(&format!("{performer} - "))
                .unwrap_or(&raw_title)
        } else {
            &raw_title
        };
        let (title, title_artists) = crate::sanitizing::title_credits(title);
        ensure!(!title.is_empty(), "Missing track title; requires review");
        let tags = &mut track.tags;
        if value(tags, "ARTIST").is_empty() {
            let credited = [value(tags, "TRACKARTIST"), value(tags, "PERFORMER")]
                .into_iter()
                .find(|v| !v.is_empty())
                .unwrap_or_else(|| release.artist.clone());
            put(tags, "ARTIST", credited);
        }
        let mut track_artists = Vec::new();
        if artist.eq_ignore_ascii_case("Various Artists") {
            if let Some(values) = tags.get("ARTIST") {
                for name in values {
                    credits::append_names(&mut track_artists, name);
                }
            }
        } else {
            credits::append_names(&mut track_artists, &artist);
        }
        for name in &album_guests {
            credits::append_names(&mut track_artists, name);
        }
        for key in ["TRACKARTIST", "ARTIST", "REMIXER"] {
            if let Some(values) = tags.get(key) {
                for name in values {
                    credits::append_names(&mut track_artists, name);
                }
            }
        }
        for name in &title_artists {
            credits::append_names(&mut track_artists, name);
        }
        let mut performers = track_artists.clone();
        if let Some(values) = tags.get("PERFORMER") {
            for name in values {
                credits::append_names(&mut performers, name);
            }
        }
        tags.insert("TRACKARTIST".into(), track_artists);
        tags.insert("PERFORMER".into(), performers);
        for key in ["ARTIST", "ALBUMARTIST", "ARTISTSORT", "ALBUMARTISTSORT"] {
            put(tags, key, &artist);
        }
        if let Some(names) = crate::sanitizing::mapped_artists(&release.artist)
            .filter(|names| names.len() > 1 && names[0].eq_ignore_ascii_case(&artist))
        {
            tags.insert("ARTIST".into(), names.to_vec());
            tags.insert("ARTISTSORT".into(), names.to_vec());
        }
        if tags.contains_key("ALBUM_ARTIST") {
            put(tags, "ALBUM_ARTIST", &artist);
        }
        if label_compilation && value(tags, "LABEL").is_empty() {
            put(tags, "LABEL", &label);
        }
        put(tags, "ALBUM", &decorated);
        put(tags, "TITLE", &title);
        put(tags, "TRACKNUMBER", number);
        put(tags, "DISCNUMBER", disc);
        for name in ["DISCTOTAL", "TOTALDISCS"] {
            put(tags, name, discs);
        }
        for name in ["TRACKTOTAL", "TOTALTRACKS"] {
            put(tags, name, totals[&disc]);
        }
        tags.remove("CATALOGNUMBER");
        tags.remove("CATALOGID");
        put(tags, "CATALOGNUMBER", &catalog);
        put(tags, "RELEASECOUNTRY", &country);
        if value(tags, "DATE").is_empty() {
            put(tags, "DATE", &release.publish);
        }
        let mut known_artists = vec![release.artist.clone(), artist.clone()];
        for key in [
            "ARTIST",
            "ALBUMARTIST",
            "TRACKARTIST",
            "PERFORMER",
            "REMIXER",
        ] {
            for name in tags.get(key).into_iter().flatten() {
                known_artists.extend(credits::artist_names(name));
            }
        }
        let genres = crate::genres::normalize(
            tags.get("GENRE")
                .into_iter()
                .flatten()
                .map(String::as_str)
                .chain(std::iter::once(release.genre.as_str())),
            &known_artists,
        );
        if !genres.is_empty() {
            tags.insert("GENRE".into(), genres);
        } else {
            tags.remove("GENRE");
        }
        if artist.eq_ignore_ascii_case("Various Artists") {
            put(tags, "COMPILATION", 1);
        }
        put(tags, "BCFUNL_ITEM_KEY", &release.key);
        put(
            tags,
            "COMMENT",
            format!(
                "bcfunl {} processed {}",
                env!("CARGO_PKG_VERSION"),
                chrono::Utc::now().date_naive()
            ),
        );
        if !release.url.is_empty() {
            put(tags, "BANDCAMP_URL", &release.url);
        }
        let mut fields = template::fields();
        for (token, value) in [
            ("artist", artist.clone()),
            ("albumartist", artist.clone()),
            ("album", album.clone()),
            ("title", title),
            ("country_iso", country.clone()),
            ("catalog_num", catalog.clone()),
            ("disc", format!("{disc:02}")),
            ("track", format!("{number:02}")),
            ("trackartist", value(tags, "TRACKARTIST")),
            ("performer", value(tags, "PERFORMER")),
            ("artistsort", value(tags, "ARTISTSORT")),
            ("albumartistsort", value(tags, "ALBUMARTISTSORT")),
            ("remixer", value(tags, "REMIXER")),
        ] {
            fields.insert(token, value);
        }
        let (dir, path) = template.render(&fields, &extension(&track.path))?;
        ensure!(
            directory.as_ref().is_none_or(|old| old == &dir),
            "Template changes release directory between tracks"
        );
        directory = Some(dir.clone());
        let relative = path.strip_prefix(dir)?.to_path_buf();
        ensure!(
            paths.insert(relative.clone()),
            "Template produced duplicate track filenames"
        );
        files.push(relative);
    }
    Ok(Plan {
        genre_fallback: None,
        artist,
        album,
        country,
        catalog,
        directory: directory.context("Empty release")?,
        files,
    })
}

fn matching_release(source: &[Track], destination: &Path) -> Result<bool> {
    let paths = collect(destination)?;
    let found = tracks(&paths)?;
    if found.len() != source.len() {
        return Ok(false);
    }
    let mut used = HashSet::new();
    for track in source {
        let signature = (disc_number(track)?, track_number(track)?);
        let candidates: Vec<_> = found
            .iter()
            .enumerate()
            .filter(|(_, other)| {
                disc_number(other).ok() == Some(signature.0)
                    && track_number(other).ok() == Some(signature.1)
                    && key(&value(&other.tags, "TITLE")) == key(&value(&track.tags, "TITLE"))
                    && (other.duration - track.duration).abs() <= 3.0
            })
            .collect();
        if candidates.len() != 1 {
            return Ok(false);
        }
        let (index, other) = candidates[0];
        if !used.insert(index) || pcm_hash(&track.path)? != pcm_hash(&other.path)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn existing(server: &Path, plan: &Plan, source: &[Track]) -> Result<Option<PathBuf>> {
    let exact = server.join(&plan.directory);
    let mut candidates = Vec::new();
    if exact.symlink_metadata().is_ok() {
        candidates.push(exact.clone());
    }
    // Older mcfunl outputs may differ only by country/catalog suffix or letter case.
    for artist in fs::read_dir(server)
        .with_context(|| format!("Scan server directory {}", server.display()))?
    {
        let artist = artist?;
        // Unrelated temporary folders can vanish while another worker finishes.
        if key(&artist.file_name().to_string_lossy()) != key(&template::safe(&plan.artist)) {
            continue;
        }
        let kind = artist
            .file_type()
            .with_context(|| format!("Inspect artist directory {}", artist.path().display()))?;
        if kind.is_symlink() || !kind.is_dir() {
            continue;
        }
        for album in fs::read_dir(artist.path())
            .with_context(|| format!("Scan artist directory {}", artist.path().display()))?
        {
            let album = album?;
            if key(base_album(&album.file_name().to_string_lossy()))
                == key(base_album(&template::safe(&plan.album)))
                && !candidates.contains(&album.path())
            {
                let kind = album.file_type().with_context(|| {
                    format!("Inspect album directory {}", album.path().display())
                })?;
                if kind.is_dir() && !kind.is_symlink() {
                    candidates.push(album.path());
                }
            }
        }
    }
    let mut matched = None;
    let mut directories = HashSet::new();
    for candidate in candidates {
        ensure!(
            candidate
                .canonicalize()
                .with_context(|| format!("Resolve existing release {}", candidate.display()))?
                .starts_with(server),
            "Existing release resolves outside --serverdir"
        );
        ensure!(
            !fs::symlink_metadata(&candidate)?.file_type().is_symlink(),
            "Server release is a symlink"
        );
        let metadata = fs::metadata(&candidate)
            .with_context(|| format!("Inspect existing release {}", candidate.display()))?;
        if metadata.ino() != 0 && !directories.insert((metadata.dev(), metadata.ino())) {
            continue;
        }
        match matching_release(source, &candidate) {
            Ok(true) => {
                ensure!(
                    matched.is_none(),
                    "Multiple verified server copies require review"
                );
                matched = Some(candidate);
            }
            Ok(false) if candidate == exact => bail!(
                "Server destination exists but audio/track verification differs; retained unchanged"
            ),
            Err(error) if candidate == exact => {
                return Err(error.context("Verify existing server destination"));
            }
            Err(error) => {
                eprintln!(
                    "  Existing candidate requires review: {}: {error:#}",
                    candidate.display()
                );
            }
            _ => {}
        }
    }
    Ok(matched)
}
fn secure_parent(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    for part in relative.components() {
        ensure!(
            matches!(part, std::path::Component::Normal(_)),
            "Unsafe publication path"
        );
        path.push(part);
        match fs::create_dir(&path) {
            Ok(()) => {
                File::open(path.parent().context("Publication ancestor")?)?.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let meta = fs::symlink_metadata(&path)?;
        ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Publication parent is not a plain directory"
        );
    }
    Ok(path)
}
fn publish(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let src = CString::new(source.as_os_str().as_bytes())?;
        let dest = CString::new(destination.as_os_str().as_bytes())?;
        let status = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                src.as_ptr(),
                libc::AT_FDCWD,
                dest.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if status != 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::EINVAL | libc::EOPNOTSUPP | libc::ENOSYS)
            ) {
                eprintln!(
                    "  Filesystem does not support no-replace rename; reserving a new destination"
                );
                publish_reserved(source, destination)?;
            } else {
                return Err(error).with_context(|| {
                    format!(
                        "Publish release without replacing existing server files: {}",
                        destination.display()
                    )
                });
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    publish_reserved(source, destination)?;
    File::open(destination.parent().context("Publication parent")?)
        .and_then(|file| file.sync_all())
        .with_context(|| format!("Sync publication parent for {}", destination.display()))?;
    Ok(())
}
fn publish_reserved(source: &Path, destination: &Path) -> Result<()> {
    // Some NAS mounts reject RENAME_NOREPLACE. Reserve only a previously absent name.
    fs::create_dir(destination).with_context(|| {
        format!(
            "Reserve new server destination without replacing existing files: {}",
            destination.display()
        )
    })?;
    let reserved = fs::metadata(destination)?;
    let result = fs::rename(source, destination).with_context(|| format!("Publish into reserved empty directory {}; NAS must support replacing an empty directory", destination.display()));
    if result.is_err() {
        // Never recursively remove: another writer may have populated the reservation.
        if fs::symlink_metadata(destination).is_ok_and(|meta| {
            !meta.file_type().is_symlink()
                && meta.dev() == reserved.dev()
                && meta.ino() == reserved.ino()
        }) {
            let _ = fs::remove_dir(destination);
        }
    }
    result
}

fn verify_tags(expected: &Tags, path: &Path) -> Result<()> {
    let actual = tagging::read_tags(path)?;
    for (key, want) in expected {
        let mut want = want.clone();
        want.sort();
        want.dedup();
        let mut found = actual.get(key).cloned().unwrap_or_default();
        found.sort();
        found.dedup();
        ensure!(
            want == found,
            "Tag verification failed for {key}: {}",
            path.display()
        );
    }
    Ok(())
}
struct Timing {
    enabled: bool,
    label: String,
    started: std::time::Instant,
    checkpoint: std::time::Instant,
    stage: &'static str,
}
impl Timing {
    fn new(release: &Release, enabled: bool) -> Self {
        let now = std::time::Instant::now();
        Self {
            enabled,
            label: format!("{} - {}", release.artist, release.album),
            started: now,
            checkpoint: now,
            stage: "source",
        }
    }
    fn next(&mut self, stage: &'static str) {
        if self.enabled {
            eprintln!(
                "  Timing [{}] {}: {:.3}s",
                self.label,
                self.stage,
                self.checkpoint.elapsed().as_secs_f64()
            );
        }
        self.checkpoint = std::time::Instant::now();
        self.stage = stage;
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        self.next("done");
        if self.enabled {
            eprintln!(
                "  Timing [{}] total: {:.3}s",
                self.label,
                self.started.elapsed().as_secs_f64()
            );
        }
    }
}

fn process_one(
    release: &Release,
    input_root: &Path,
    server: &Path,
    template: &template::Template,
    library: Option<&genres::Library>,
    timings: bool,
) -> Result<(PathBuf, Plan, bool)> {
    let mut timing = Timing::new(release, timings);
    let meta = fs::symlink_metadata(&release.source).context("Read downloaded source")?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink(),
        "Downloaded source is not a regular file"
    );
    let source = release.source.canonicalize()?;
    ensure!(
        source.starts_with(input_root),
        "Downloaded source is outside --output"
    );
    timing.next("extract_local");
    let work = tempfile::tempdir_in(input_root)?;
    let extraction = work.path().join("extracted");
    fs::create_dir(&extraction)?;
    let paths = if extension(&source) == "zip" {
        extract(&source, &extraction)?
    } else {
        ensure!(audio(&source), "Unsupported downloaded payload");
        vec![source]
    };
    ensure!(
        !paths.iter().any(|p| extension(p) == "cue"),
        "CUE-based releases require the existing mcfunl processor"
    );
    timing.next("read_tags");
    let mut tracks = tracks(&paths)?;
    timing.next("prepare_tags");
    let mut plan = prepare_tags(release, &mut tracks, template)?;
    timing.next("verify_existing");
    if let Some(existing) = existing(server, &plan, &tracks)? {
        return Ok((existing, plan, true));
    }
    timing.next("genre_fallback");
    if let Some(library) = library {
        plan.genre_fallback = library.apply(&plan.artist, &mut tracks)?;
    }
    timing.next("copy_tag_verify");
    let output_work = tempfile::tempdir_in(server)?;
    let staging = output_work.path().join("publish");
    fs::create_dir(&staging)?;
    let mut parts = [std::time::Duration::ZERO; 6];
    let validation = (|| -> Result<()> {
        for (track, relative) in tracks.iter().zip(&plan.files) {
            shutdown::check()?;
            let dest = staging.join(relative);
            measured(&mut parts[0], || {
                fs::create_dir_all(dest.parent().context("Track parent")?)?;
                fs::copy(&track.path, &dest)?;
                Ok(())
            })?;
            measured(&mut parts[1], || tagging::write(&dest, &track.tags))?;
            let source_hash = measured(&mut parts[2], || pcm_hash(&track.path))?;
            let destination_hash = measured(&mut parts[3], || pcm_hash(&dest))?;
            ensure!(
                source_hash == destination_hash,
                "Decoded audio changed during tagging: {}",
                dest.display()
            );
            measured(&mut parts[4], || verify_tags(&track.tags, &dest))?;
            measured(&mut parts[5], || {
                File::open(&dest)?.sync_all()?;
                Ok(())
            })?;
        }
        Ok(())
    })();
    if timings {
        for (name, elapsed) in [
            "copy",
            "write_tags",
            "source_pcm_hash",
            "destination_pcm_hash",
            "tag_readback",
            "file_sync",
        ]
        .iter()
        .zip(parts)
        {
            eprintln!(
                "  Timing [{} - {}] {name}: {:.3}s",
                release.artist,
                release.album,
                elapsed.as_secs_f64()
            );
        }
    }
    validation?;
    timing.next("artwork");
    if let Some(cover) = paths
        .iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|s| s.eq_ignore_ascii_case("cover.jpg"))
        })
        .or_else(|| {
            paths.iter().find(|path| {
                path.file_name()
                    .is_some_and(|s| s.eq_ignore_ascii_case("folder.jpg"))
            })
        })
    {
        ensure!(
            !plan.files.contains(&PathBuf::from("folder.jpg")),
            "Artwork path collision"
        );
        fs::copy(cover, staging.join("folder.jpg"))?;
        File::open(staging.join("folder.jpg"))?.sync_all()?;
    }
    if !staging.join("folder.jpg").exists() {
        for track in &tracks {
            if let Some(picture) = tagging::picture(&track.path)? {
                fs::write(staging.join("folder.jpg"), picture)?;
                File::open(staging.join("folder.jpg"))?.sync_all()?;
                break;
            }
        }
    }
    timing.next("manifest_and_sync");
    let manifest = staging.join(".bcfunl-release.json");
    fs::write(
        &manifest,
        serde_json::to_vec_pretty(
            &json!({"item_key":release.key,"artist":plan.artist,"album":plan.album,"country_iso":plan.country,"files":plan.files,"version":env!("CARGO_PKG_VERSION"),"genre_fallback":plan.genre_fallback}),
        )?,
    )?;
    File::open(manifest)?.sync_all()?;
    // Sync nested track directories before making the complete release visible.
    for path in plan
        .files
        .iter()
        .filter_map(|p| p.parent())
        .collect::<HashSet<_>>()
    {
        File::open(staging.join(path))?.sync_all()?;
    }
    File::open(&staging)?.sync_all()?;
    secure_parent(server, plan.directory.parent().context("Release parent")?)?;
    let destination = server.join(&plan.directory);
    shutdown::check()?;
    timing.next("publish");
    publish(&staging, &destination)?;
    timing.next("temporary_cleanup");
    drop(output_work);
    drop(work);
    Ok((destination, plan, false))
}

fn completed_here(path: &str, server: &Path, item_key: &str) -> bool {
    let path = Path::new(path);
    if !path.canonicalize().is_ok_and(|p| p.starts_with(server)) {
        return false;
    }
    if !path
        .symlink_metadata()
        .is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink())
    {
        return false;
    }
    let Ok(text) = fs::read_to_string(path.join(".bcfunl-release.json")) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    manifest["item_key"].as_str() == Some(item_key)
        && manifest["files"].as_array().is_some_and(|files| {
            !files.is_empty()
                && files.iter().all(|file| {
                    file.as_str().is_some_and(|relative| {
                        let relative = Path::new(relative);
                        !relative.is_absolute()
                            && relative
                                .components()
                                .all(|part| matches!(part, std::path::Component::Normal(_)))
                            && path
                                .join(relative)
                                .canonicalize()
                                .is_ok_and(|p| p.starts_with(path))
                            && path.join(relative).symlink_metadata().is_ok_and(|meta| {
                                meta.is_file() && !meta.file_type().is_symlink() && meta.len() > 0
                            })
                    })
                })
        })
}

fn processing_rows(db: &Database, args: &Args) -> Result<Vec<crate::db::Row>> {
    let mut sql = String::from(
        "SELECT item_key,artist,COALESCE(sanitized_title,release_title),file_path,genre,country_iso,catalog_num,publish_date,release_url,process_date,processed_path,process_error FROM releases WHERE user=? AND download_date IS NOT NULL",
    );
    let mut values = vec![args.user.clone()];
    if let Some(artist) = &args.artist {
        sql.push_str(" AND UPPER(artist) LIKE '%' || ? || '%'");
        values.push(crate::sanitizing::artist_display(artist).to_uppercase());
    }
    if let Some(from) = args.fromdt {
        sql.push_str(" AND purchase_date>=?");
        values.push(from.to_string());
    }
    if let Some(to) = args.todt {
        sql.push_str(" AND purchase_date<=?");
        values.push(to.to_string());
    }
    sql.push_str(" ORDER BY purchase_date DESC,item_key");
    db.query(&sql, &values.iter().map(String::as_str).collect::<Vec<_>>())
}

pub(crate) use crate::staging::lock as staging_lock;

pub fn run(args: &Args) -> Result<()> {
    let started = std::time::Instant::now();
    crate::sanitizing::load_artist_map()?;
    let template = template::Template::new(args.template.as_deref().unwrap_or(template::DEFAULT))?;
    let input = args
        .output
        .canonicalize()
        .context("Open existing --output staging directory")?;
    let _staging_lock = staging_lock(&input)?;
    let db_path = input.join("collection.sqlite3");
    ensure!(
        db_path.is_file(),
        "--process requires an existing collection.sqlite3 in --output"
    );
    let server = args
        .serverdir
        .as_ref()
        .context("--process requires --serverdir")?;
    fs::create_dir_all(server)?;
    let server = server.canonicalize()?;
    ensure!(
        !server.starts_with(&input) && !input.starts_with(&server),
        "--serverdir and --output must be separate directory trees"
    );
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(server.join(".bcfunl-process.lock"))?;
    lock.try_lock()
        .context("Another processor holds the server directory lock")?;
    let db = Database::open(db_path.to_str().context("Database path is not UTF-8")?)?;
    let library_path = args
        .library
        .clone()
        .unwrap_or_else(|| input.join("resources/library.sqlite3"));
    let library_path = if args.library.is_some() || library_path.exists() {
        genres::Library::open(&library_path)?;
        Some(library_path)
    } else {
        None
    };
    let rows = processing_rows(&db, args)?;
    if args.timings {
        eprintln!(
            "Timing: bcfunl v{} setup/query {:.3}s, {} eligible history rows",
            env!("CARGO_PKG_VERSION"),
            started.elapsed().as_secs_f64(),
            rows.len()
        );
    }
    let mut attempted_keys = HashSet::new();
    let mut history_time = std::time::Duration::ZERO;
    let mut attempted = 0;
    let mut failures = 0;
    let mut published = 0;
    let mut skipped = 0;
    let limit = usize::from(args.parallel.unwrap_or(1));
    thread::scope(|scope| -> Result<()> {
        let mut active = std::collections::VecDeque::new();
        let mut record = |release: &Release,
                          result: Result<(PathBuf, Plan, bool)>,
                          skipped: &mut u32|
         -> Result<()> {
            match result {
                Ok((path, plan, exists)) => {
                    db.query("UPDATE releases SET process_date=?,process_error=NULL,processed_path=?,catalog_num=COALESCE(NULLIF(?,''),catalog_num),genre_fallback=COALESCE(NULLIF(?,''),genre_fallback) WHERE user=? AND item_key=?", &[&chrono::Utc::now().to_rfc3339(),&path.to_string_lossy(),&plan.catalog,&plan.genre_fallback.as_ref().map(ToString::to_string).unwrap_or_default(),&args.user,&release.key])?;
                    if exists {
                        *skipped += 1;
                        eprintln!(
                            "  Verified existing {} - {}; skipped",
                            release.artist, release.album
                        );
                    } else {
                        published += 1;
                        eprintln!("  Published {}", path.display());
                    }
                }
                Err(_) if shutdown::requested() => {}
                Err(error) => {
                    let message = format!("{error:#}");
                    eprintln!(
                        "  Failed: {} - {}: {message}",
                        release.artist, release.album
                    );
                    failures += 1;
                    db.query(
                        "UPDATE releases SET process_error=? WHERE user=? AND item_key=?",
                        &[&message, &args.user, &release.key],
                    )?;
                }
            }
            Ok(())
        };
        let input = &input;
        let server = &server;
        let template = &template;
        let library_path = &library_path;
        for row in rows {
            if shutdown::requested() {
                break;
            }
            if args.count.is_some_and(|limit| attempted >= limit) {
                break;
            }
            let history_started = std::time::Instant::now();
            let completed = row[9].is_some()
                && !args.force
                && row[10].as_deref().is_some_and(|p| {
                    completed_here(p, server, row[0].as_deref().unwrap_or_default())
                });
            history_time += history_started.elapsed();
            if completed {
                skipped += 1;
                continue;
            }
            attempted += 1;
            let get = |i: usize| row[i].clone().unwrap_or_default();
            let release = Release {
                key: get(0),
                artist: get(1),
                album: get(2),
                source: PathBuf::from(get(3)),
                genre: get(4),
                country: get(5),
                catalog: get(6),
                publish: get(7),
                url: get(8),
            };
            eprintln!(
                "[{attempted}] Process {} - {}",
                release.artist, release.album
            );
            attempted_keys.insert(release.key.clone());
            let worker_release = release.clone();
            active.push_back((
                release,
                scope.spawn(move || {
                    let library = library_path
                        .as_deref()
                        .map(genres::Library::open)
                        .transpose()?;
                    process_one(
                        &worker_release,
                        input,
                        server,
                        template,
                        library.as_ref(),
                        args.timings,
                    )
                }),
            ));
            if active.len() == limit {
                let (release, task) = active.pop_front().unwrap();
                let result = task
                    .join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("Processing worker panicked")));
                record(&release, result, &mut skipped)?;
            }
        }
        for (release, task) in active {
            let result = task
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("Processing worker panicked")));
            record(&release, result, &mut skipped)?;
        }
        Ok(())
    })?;
    if args.timings {
        eprintln!(
            "Timing: history skip checks {:.3}s; processing wall {:.3}s",
            history_time.as_secs_f64(),
            started.elapsed().as_secs_f64()
        );
    }
    if args.summary {
        println!(
            "Processing: {attempted} attempted, {published} published, {skipped} skipped, {failures} failed"
        );
    }
    shutdown::check()?;
    if args.clean {
        let cleanup_started = std::time::Instant::now();
        let result = clean::completed(&db, args, &input, Some(&attempted_keys));
        if args.timings {
            eprintln!(
                "Timing: cleanup {:.3}s",
                cleanup_started.elapsed().as_secs_f64()
            );
        }
        if let Err(error) = result {
            if failures > 0 {
                bail!("{failures} releases failed processing; cleanup also failed: {error:#}");
            }
            return Err(error);
        }
    }
    ensure!(
        failures == 0,
        "{failures} releases failed processing; failed sources and existing server files retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn cancellation_fixture(root: &Path) -> Result<()> {
    let _staging = tempfile::tempdir_in(root)?;
    command(
        Command::new("sh")
            .args([
                "-c",
                "echo $$ > \"$1/child-pid\"; touch \"$1/ready\"; exec sleep 60",
                "fixture",
            ])
            .arg(root),
    )?;
    Ok(())
}
