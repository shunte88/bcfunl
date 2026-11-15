//! In-place collection tag maintenance using read-only Discogs snapshots.
#[cfg(test)]
use super::tempfile;
use super::{Release, Tags, Track, audio, common, credits, discogs, tagging, value};
use crate::{Args, db::Database, shutdown};
use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, FileTimes, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

// Explicit unlock prevents a concurrently forked helper from extending a lock's lifetime.
struct FileLock(File);
impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn names(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|v| credits::artist_names(v))
        .map(|v| crate::sanitizing::artist_display(&v))
        .flat_map(|v| credits::artist_names(&v))
        .fold(Vec::new(), |mut all, name| {
            if !all.iter().any(|old| old.eq_ignore_ascii_case(&name)) {
                all.push(name);
            }
            all
        })
}
fn artists(body: &Value) -> Vec<String> {
    names(
        &body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| {
                a["anv"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| a["name"].as_str())
            })
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    )
}
fn append(tags: &mut Tags, key: &str, values: Vec<String>) {
    let values = names(&values);
    if values.is_empty() {
        return;
    }
    let current = tags.entry(key.into()).or_default();
    for value in values {
        if !current.iter().any(|old| old.eq_ignore_ascii_case(&value)) {
            current.push(value);
        }
    }
}
fn credit_tags(tags: &mut Tags, body: &Value) {
    for (tag, roles) in [
        ("PERFORMER", &["Performer"][..]),
        ("COMPOSER", &["Composer", "Composed By"][..]),
        ("CONDUCTOR", &["Conductor", "Conducted By"][..]),
        ("LYRICIST", &["Lyrics By"][..]),
        ("WRITER", &["Written By", "Written-By"][..]),
        ("PRODUCER", &["Producer", "Produced By"][..]),
        ("ENGINEER", &["Engineer", "Engineered By"][..]),
        ("ARRANGER", &["Arranged By"][..]),
        ("REMIXER", &["Remix", "Remixed By"][..]),
        ("MIXER", &["Mixed By"][..]),
    ] {
        append(tags, tag, discogs::album_credits(body, roles));
    }
}
fn vocabulary(db: &Database) -> Result<BTreeMap<String, String>> {
    let rows = db
        .query("SELECT genre FROM sanitized_genres ORDER BY genre", &[])
        .context("Rebuild --resources first: collection mode requires sanitized_genres")?;
    ensure!(
        !rows.is_empty(),
        "Discogs sanitized genre vocabulary is empty; rebuild --resources first"
    );
    Ok(rows
        .into_iter()
        .filter_map(|row| row[0].clone())
        .map(|name| (name.to_lowercase(), name))
        .collect())
}
fn clean_genres(tags: &mut Tags, allowed: &BTreeMap<String, String>, artist: &[String]) {
    let genres = crate::genres::normalize(tags.get("GENRE").into_iter().flatten(), artist)
        .into_iter()
        .filter_map(|genre| allowed.get(&genre.to_lowercase()).cloned())
        .collect::<Vec<_>>();
    if genres.is_empty() {
        tags.remove("GENRE");
    } else {
        tags.insert("GENRE".into(), genres);
    }
}
fn clean(
    tags: &mut Tags,
    album_artist: &str,
    found: Option<&discogs::Match>,
    allowed: &BTreeMap<String, String>,
) {
    for key in [
        "ARTIST",
        "TRACKARTIST",
        "ALBUMARTIST",
        "ALBUMARTISTSORT",
        "ARTISTSORT",
        "PERFORMER",
        "COMPOSER",
        "CONDUCTOR",
        "REMIXER",
        "LYRICIST",
        "WRITER",
        "PRODUCER",
        "ENGINEER",
        "ARRANGER",
        "MIXER",
    ] {
        if let Some(values) = tags.get(key).cloned() {
            let values = if key.ends_with("SORT") {
                let mut fields = Tags::from([(key.to_owned(), values)]);
                credits::split_fields(&mut fields);
                fields
                    .remove(key)
                    .unwrap_or_default()
                    .iter()
                    .map(|name| crate::sanitizing::artist_display(name))
                    .collect()
            } else {
                names(&values)
            };
            tags.insert(key.into(), values);
        }
    }
    if !album_artist.is_empty() && value(tags, "ALBUMARTIST").is_empty() {
        tags.insert("ALBUMARTIST".into(), names(&[album_artist.into()]));
    }
    if let Some(found) = found {
        let body = &found.body;
        credit_tags(tags, body);
        let album = artists(&body["artists"]);
        if !album.is_empty() {
            tags.insert("ALBUMARTIST".into(), album);
        }
        let title = value(tags, "TITLE");
        let tracks = body["tracklist"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| {
                t["title"]
                    .as_str()
                    .is_some_and(|s| crate::discogs_identity::same(s, &title))
            })
            .collect::<Vec<_>>();
        if tracks.len() == 1 {
            let names = artists(&tracks[0]["artists"]);
            if !names.is_empty() {
                tags.insert("TRACKARTIST".into(), names.clone());
                tags.insert("ARTIST".into(), names);
            }
            credit_tags(tags, tracks[0]);
            let position = tracks[0]["position"].as_str().unwrap_or_default();
            // Scoped credits require an exact position; ambiguous ranges stay untouched.
            let entries = body["extraartists"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|a| {
                    !position.is_empty()
                        && a["tracks"].as_str().is_some_and(|s| {
                            s.split(',')
                                .any(|part| part.trim().eq_ignore_ascii_case(position))
                        })
                })
                .cloned()
                .map(|mut a| {
                    a["tracks"] = Value::String(String::new());
                    a
                })
                .collect::<Vec<_>>();
            credit_tags(tags, &serde_json::json!({"extraartists":entries}));
        }
        for (key, field) in [("DATE", "released"), ("RELEASECOUNTRY", "country")] {
            if let Some(text) = body[field].as_str().filter(|s| !s.trim().is_empty())
                && (key != "DATE" || chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok())
            {
                let text = if key == "RELEASECOUNTRY" {
                    crate::metadata::country(text).unwrap_or(text)
                } else {
                    text
                };
                tags.insert(key.into(), vec![text.into()]);
            }
        }
        if value(tags, "DATE").is_empty()
            && let Some(year) = body["released"]
                .as_str()
                .and_then(|s| s.get(..4))
                .filter(|s| s.parse::<u16>().is_ok_and(|y| (1000..=2999).contains(&y)))
        {
            tags.insert("DATE".into(), vec![year.into()]);
        }
        if value(tags, "CATALOGNUMBER").is_empty()
            && let Some(cat) = &found.catalog
        {
            tags.insert("CATALOGNUMBER".into(), vec![cat.clone()]);
            tags.insert("CATALOGNO".into(), vec![cat.clone()]);
        }

        if let Some(label) = body["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|l| l["name"].as_str())
            .filter(|s| !s.trim().is_empty())
        {
            tags.insert("LABEL".into(), vec![label.into()]);
        }
    }
    for key in ["ALBUM", "TITLE"] {
        let text = value(tags, key);
        if !text.is_empty() {
            tags.insert(key.into(), vec![crate::sanitizing::display(&text)]);
        }
    }
    for (from, to) in [("ARTIST", "TRACKARTIST"), ("TRACKARTIST", "ARTIST")] {
        if value(tags, to).is_empty()
            && let Some(values) = tags.get(from).cloned()
        {
            tags.insert(to.into(), values);
        }
    }
    for (from, to) in [("ALBUMARTIST", "ALBUMARTISTSORT"), ("ARTIST", "ARTISTSORT")] {
        if value(tags, to).is_empty()
            && let Some(values) = tags.get(from).cloned()
        {
            tags.insert(to.into(), values);
        }
    }
    if let Some(performers) = tags.get("TRACKARTIST").cloned() {
        append(tags, "PERFORMER", performers);
    }
    if let Some(year) = value(tags, "DATE")
        .get(..4)
        .filter(|s| s.parse::<u16>().is_ok_and(|y| (1000..=2999).contains(&y)))
    {
        tags.insert("YEAR".into(), vec![year.into()]);
    }
    let code = discogs::country(&value(tags, "RELEASECOUNTRY"));
    let code = if code.is_empty() {
        discogs::country(&value(tags, "COUNTRY_ISO"))
    } else {
        code
    };
    if !code.is_empty() {
        tags.insert("COUNTRY_ISO".into(), vec![code.clone()]);
        tags.insert("RELEASECOUNTRY".into(), vec![code.clone()]);
        if tags.contains_key("COUNTRY") {
            tags.insert("COUNTRY".into(), vec![code]);
        }
    }
    let cat = value(tags, "CATALOGNUMBER");
    if !cat.is_empty() {
        tags.insert("CATALOGNO".into(), vec![cat.clone()]);
        if tags.contains_key("CATALOGID") {
            tags.insert("CATALOGID".into(), vec![cat]);
        }
    }
    if tags.contains_key("ALBUM_ARTIST")
        && let Some(names) = tags.get("ALBUMARTIST").cloned()
    {
        tags.insert("ALBUM_ARTIST".into(), names);
    }
    let names = tags.get("ARTIST").cloned().unwrap_or_default();
    clean_genres(tags, allowed, &names);
}

fn preserve_times(
    file: &File,
    before: &fs::Metadata,
    edit: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let times = FileTimes::new()
        .set_accessed(before.accessed()?)
        .set_modified(before.modified()?);
    let _unit = shutdown::unit_task();
    let result = edit();
    let restored = file
        .set_times(times)
        .context("Restore original media timestamps");
    match (result, restored) {
        (Err(edit), Err(restore)) => anyhow::bail!("{edit:#}; {restore:#}"),
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    }
}

fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.dev() == b.dev()
        && a.ino() == b.ino()
        && a.len() == b.len()
        && a.modified().ok() == b.modified().ok()
        && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}
fn update(path: &Path, original: &Tags, desired: &Tags) -> Result<bool> {
    shutdown::check()?;
    if original == desired && tagging::raw_tags(path)? == *desired {
        return Ok(false);
    }
    let source = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    source
        .try_lock()
        .context("Another writer holds the media file lock")?;
    let locked = FileLock(source);
    let source = &locked.0;
    let before = source.metadata()?;
    ensure!(
        before.is_file() && before.nlink() == 1 && !before.permissions().readonly(),
        "Read-only, nonregular, or hard-linked media requires review"
    );
    ensure!(
        tagging::read_metadata(path)? == *original,
        "Media tags changed before collection update; original retained"
    );
    ensure!(
        same_file(&before, &fs::symlink_metadata(path)?),
        "Media changed before collection update"
    );
    shutdown::check()?;
    preserve_times(source, &before, || {
        tagging::write(path, desired)?;
        let actual = tagging::read_metadata(path)?;
        for (key, values) in desired {
            ensure!(
                actual.get(key) == Some(values),
                "Tag readback failed after direct update: {key}"
            );
        }
        ensure!(
            actual.get("GENRE") == desired.get("GENRE"),
            "Discarded genres remain after direct update"
        );
        Ok(())
    })?;
    Ok(true)
}

fn folder_hints(tags: &mut Tags, path: &Path) {
    let artist = value(tags, "ALBUMARTIST");
    let artist = if artist.is_empty() {
        value(tags, "ARTIST")
    } else {
        artist
    };
    let album = value(tags, "ALBUM");
    let metadata = crate::sanitizing::release_hints(&album, &artist);
    let mut folder = path.parent().unwrap_or(Path::new(""));
    let name = folder
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace(' ', "");
    if ["cd", "disc", "disk"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|tail| !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()))
    }) {
        folder = folder.parent().unwrap_or(folder);
    }
    let directory = crate::sanitizing::release_hints(
        &folder.file_name().unwrap_or_default().to_string_lossy(),
        &artist,
    );
    let hint_country = directory.country.as_ref().or(metadata.country.as_ref());
    let current = discogs::country(&value(tags, "RELEASECOUNTRY"));
    if let Some(country) = hint_country
        && !current.is_empty()
        && current != *country
    {
        eprintln!(
            "  Folder country {country} differs from existing {current}; retaining existing tag: {}",
            path.display()
        );
    }
    let code = if current.is_empty() {
        hint_country.cloned().unwrap_or_default()
    } else {
        current
    };
    if !code.is_empty() {
        tags.insert("RELEASECOUNTRY".into(), vec![code.clone()]);
        tags.insert("COUNTRY_ISO".into(), vec![code.clone()]);
        if tags.contains_key("COUNTRY") {
            tags.insert("COUNTRY".into(), vec![code]);
        }
    }
    let hint_catalog = directory
        .catalog
        .as_ref()
        .or(metadata.catalog.as_ref())
        .cloned();
    let current = value(tags, "CATALOGNUMBER");
    if let Some(catalog) = hint_catalog {
        if current.is_empty() {
            tags.insert("CATALOGNUMBER".into(), vec![catalog.clone()]);
            tags.insert("CATALOGNO".into(), vec![catalog]);
        } else {
            let key = |s: &str| {
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect::<String>()
            };
            if key(&current) != key(&catalog) {
                eprintln!(
                    "  Folder catalog {catalog} differs from existing {current}; retaining existing tag: {}",
                    path.display()
                );
            }
        }
    }
    let malformed = album.matches('[').count() != album.matches(']').count();
    let title = if (malformed || album.is_empty())
        && (directory.country.is_some() || directory.catalog.is_some() || malformed)
    {
        directory.title
    } else {
        metadata.title
    };
    if !title.is_empty() {
        tags.insert("ALBUM".into(), vec![title]);
    }
}

struct Walked {
    identities: Vec<(String, String)>,
    changed: usize,
    matched: usize,
}
fn release(paths: &[PathBuf], library: &Path) -> Result<Walked> {
    let db = Database::read_only(library.to_str().context("Library path is not UTF-8")?)?;
    let allowed = vocabulary(&db)?;
    let mut groups: BTreeMap<String, Vec<Track>> = BTreeMap::new();
    let mut originals = BTreeMap::new();
    for path in paths {
        shutdown::check()?;
        let mut tags = tagging::read_metadata(path)?;
        originals.insert(path.clone(), tags.clone());
        folder_hints(&mut tags, path);
        groups
            .entry(value(&tags, "ALBUM"))
            .or_default()
            .push(Track {
                path: path.clone(),
                tags,
                duration: 0.0, // Not inspected or used for collection metadata work.
            });
    }
    let mut changed = 0;
    let mut matched = 0;
    let mut identities = Vec::new();
    for (album, mut tracks) in groups {
        let artist = common(&tracks, "ALBUMARTIST").unwrap_or_default();
        let artist = if artist.is_empty() {
            common(&tracks, "ARTIST").unwrap_or_default()
        } else {
            artist
        };
        let info = Release {
            key: String::new(),
            artist: artist.clone(),
            album: album.clone(),
            source: paths[0].clone(),
            genre: String::new(),
            country: common(&tracks, "RELEASECOUNTRY").unwrap_or_default(),
            catalog: String::new(),
            publish: String::new(),
            url: String::new(),
        };
        let found = if artist.is_empty()
            || album.is_empty()
            || common(&tracks, "ALBUMARTIST").is_err()
            || common(&tracks, "RELEASECOUNTRY").is_err()
        {
            None
        } else {
            discogs::find_collection(&db, &info, &tracks)?
        };
        let original = tracks
            .iter()
            .map(|t| originals.remove(&t.path).expect("original track tags"))
            .collect::<Vec<_>>();
        if let Some(found) = &found {
            discogs::apply(&info, &mut tracks, found);
            matched += 1;
        }
        for (track, before) in tracks.iter_mut().zip(original) {
            clean(&mut track.tags, &artist, found.as_ref(), &allowed);
            changed += usize::from(
                update(&track.path, &before, &track.tags)
                    .with_context(|| format!("Update {}", track.path.display()))?,
            );
        }
        identities.push((
            crate::sanitizing::artist_display(&artist),
            crate::sanitizing::display(&album),
        ));
    }
    Ok(Walked {
        identities,
        changed,
        matched,
    })
}

fn cutoff(args: &Args, today: chrono::NaiveDate) -> Result<chrono::NaiveDate> {
    args.fromdt.map(Ok).unwrap_or_else(|| {
        today
            .checked_sub_months(chrono::Months::new(2))
            .context("Cannot calculate collection date window")
    })
}
fn modified_after(latest: chrono::DateTime<chrono::Utc>, from: chrono::NaiveDate) -> bool {
    latest.date_naive() > from
}
fn already_walked(
    history: &Database,
    folder: &Path,
    latest: chrono::DateTime<chrono::Utc>,
) -> Result<bool> {
    let rows = history.query(
        "SELECT MAX(walk_date) FROM collection WHERE path=?",
        &[folder.to_str().context("Collection path is not UTF-8")?],
    )?;
    match rows.first().and_then(|row| row[0].as_deref()) {
        Some(date) => Ok(chrono::DateTime::parse_from_rfc3339(date)
            .context("Invalid collection walk_date")?
            .with_timezone(&chrono::Utc)
            >= latest),
        None => Ok(false),
    }
}

pub(crate) fn run(args: &Args) -> Result<()> {
    let root = args
        .serverdir
        .as_ref()
        .context("--collection requires --serverdir")?
        .canonicalize()?;
    ensure!(root.is_dir(), "--serverdir must be a music directory");
    let library = args
        .library
        .clone()
        .unwrap_or_else(|| args.output.join("resources/library.sqlite3"));
    let db = Database::read_only(library.to_str().context("Library path is not UTF-8")?)?;
    vocabulary(&db)?;
    db.query("SELECT id,title_key,data FROM releases LIMIT 0", &[])
        .context("Rebuild --resources with detailed releases first")?;
    drop(db);
    fs::create_dir_all(&args.output)?;
    let history_root = args.output.canonicalize()?;
    ensure!(
        !history_root.starts_with(&root) && !root.starts_with(&history_root),
        "--output history and --serverdir music must be separate directory trees"
    );
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(root.join(".bcfunl-process.lock"))?;
    lock.try_lock()
        .context("Another process or collection job holds this music root")?;
    let _music_lock = FileLock(lock);
    let _history_lock = crate::staging::shared(&history_root)?;
    let history_path = args.output.join("collection.sqlite3");
    let history = Database::open(history_path.to_str().context("History path is not UTF-8")?)?;
    let from = cutoff(args, chrono::Utc::now().date_naive())?;
    eprintln!(
        "Collection window: media modified after {from} (UTC); --force does not bypass this cutoff"
    );
    let mut outside = 0;
    let mut pending = vec![root];
    let mut folders = Vec::new();
    while let Some(folder) = pending.pop() {
        shutdown::check()?;
        let mut files = Vec::new();
        let mut latest = None;
        for entry in fs::read_dir(&folder)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if !entry.file_name().to_string_lossy().starts_with(".bcfunl-") {
                    pending.push(entry.path());
                }
            } else if kind.is_file() && audio(&entry.path()) {
                let modified = chrono::DateTime::<chrono::Utc>::from(entry.metadata()?.modified()?);
                latest = Some(
                    latest.map_or(modified, |previous: chrono::DateTime<chrono::Utc>| {
                        previous.max(modified)
                    }),
                );
                files.push(entry.path());
            }
        }
        files.sort();
        if let Some(latest) = latest {
            if modified_after(latest, from) {
                folders.push((folder, files, latest));
            } else {
                outside += 1;
            }
        }
    }
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    let mut attempted = 0;
    let mut changed = 0;
    let mut matches = 0;
    let mut failures = 0;
    let mut skipped = 0;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(usize::from(args.parallel.unwrap_or(3)))
        .build()?;
    // Keep dispatch bounded so a stop never leaves an unbounded queue of releases.
    for chunk in folders.chunks(usize::from(args.parallel.unwrap_or(3))) {
        shutdown::check()?;
        let mut jobs = Vec::new();
        for (folder, files, latest) in chunk {
            if args.count.is_some_and(|n| attempted >= n) {
                break;
            }
            if !args.force && already_walked(&history, folder, *latest)? {
                skipped += 1;
                continue;
            }
            if args.artist.as_ref().is_some_and(|artist| {
                !folder
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&artist.to_lowercase())
            }) {
                continue;
            }
            attempted += 1;
            jobs.push((folder, files));
        }
        let results = pool.install(|| {
            jobs.par_iter()
                .map(|(folder, files)| (folder, release(files, &library)))
                .collect::<Vec<_>>()
        });
        for (folder, result) in results {
            match result {
                Ok(Walked {
                    identities,
                    changed: n,
                    matched: m,
                }) => {
                    for (artist, title) in identities {
                        history.query("INSERT INTO collection VALUES(?,?,?,?) ON CONFLICT(path,artist,title) DO UPDATE SET walk_date=excluded.walk_date",&[folder.to_str().unwrap(),&artist,&title,&chrono::Utc::now().to_rfc3339()])?;
                    }
                    changed += n;
                    matches += m;
                    println!(
                        "  Walked {}: {n} files updated, {m} Discogs matches",
                        folder.display()
                    );
                }
                Err(error) if shutdown::requested() => return Err(error),
                Err(error) => {
                    failures += 1;
                    eprintln!("  Failed collection {}: {error:#}", folder.display());
                }
            }
        }
        if args.count.is_some_and(|n| attempted >= n) {
            break;
        }
    }
    println!(
        "Collection: {attempted} folders attempted, {changed} files updated, {matches} matched, {skipped} skipped, {outside} outside date window, {failures} failed"
    );
    ensure!(
        failures == 0,
        "{failures} collection folders failed; direct updates may have completed on individual files; failed folders remain retryable"
    );
    Ok(())
}

#[cfg(test)]
pub(crate) fn cancellation_fixture(root: &Path) -> Result<()> {
    let path = root.join("metadata-unit");
    fs::write(&path, b"before")?;
    let file = File::open(&path)?;
    let before = file.metadata()?;
    preserve_times(&file, &before, || {
        fs::write(&path, b"updated")?;
        fs::write(root.join("ready"), b"ready")?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !shutdown::requested() {
            ensure!(std::time::Instant::now() < deadline, "Stop did not arrive");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        shutdown::check()?;
        assert!(
            std::thread::spawn(|| shutdown::check().is_err())
                .join()
                .unwrap()
        );
        fs::write(root.join("unit-finished"), b"finished")?;
        Ok(())
    })?;
    ensure!(
        file.metadata()?.modified()? == before.modified()?,
        "Timestamp not restored after stop"
    );
    shutdown::check()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serde_json::json;
    use std::process::Command;
    fn library(path: &Path) {
        let db = Database::connect(path.to_str().unwrap()).unwrap();
        for sql in [
            "CREATE TABLE sanitized_genres(genre TEXT PRIMARY KEY)",
            "INSERT INTO sanitized_genres VALUES('Electronic'),('Ambient'),('EBM')",
            "CREATE TABLE releases(id INTEGER PRIMARY KEY,title_key TEXT,data TEXT)",
        ] {
            db.query(sql, &[]).unwrap();
        }
        let body = json!({"title":"Proof","artists":[{"name":"Romy"}],"country":"UK","released":"2020-02-03","labels":[{"catno":"CAT01","name":"Fixture Label"}],"genres":["Electronic"],"styles":["Ambient"],"extraartists":[{"name":"Composer One","role":"Composed By"},{"name":"Conductor One","role":"Conductor"}],"tracklist":[{"position":"A1","title":"One","artists":[{"name":"Guest"}],"extraartists":[{"name":"Track Player","role":"Performer"}]}]});
        db.query(
            "INSERT INTO releases VALUES(1,'proof',?)",
            &[&body.to_string()],
        )
        .unwrap();
    }
    fn media(path: &Path, artist: &str, album: &str) {
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.1",
        ]);
        if path.extension().unwrap() == "m4a" {
            cmd.args(["-c:a", "alac"]);
        }
        super::super::command(cmd.arg(path)).unwrap();
        let tags = Tags::from([
            ("ALBUM".into(), vec![album.into()]),
            ("ARTIST".into(), vec![artist.into()]),
            ("TITLE".into(), vec!["One".into()]),
            (
                "GENRE".into(),
                vec!["EBM - electronic body music;Unlisted;123;A".into()],
            ),
            ("CUSTOM_VENDOR".into(), vec!["Keep This Value".into()]),
        ]);
        tagging::write(path, &tags).unwrap();
    }
    #[test]
    fn unmatched_formats_preserve_audio_artwork_names_custom_tags_and_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let allowed = BTreeMap::from([
            ("ebm".into(), "EBM".into()),
            ("electronic".into(), "Electronic".into()),
        ]);
        for ext in ["flac", "mp3", "wav", "aiff", "m4a", "ogg"] {
            let path = dir.path().join(format!("Untouched Filename.{ext}"));
            media(&path, "the xx", "Unknown Album");
            let stamp = std::time::UNIX_EPOCH + std::time::Duration::new(1_600_000_000, 123456789);
            File::open(&path).unwrap().set_modified(stamp).unwrap();
            let before = tagging::read(&path).unwrap().0;
            let hash = super::super::pcm_hash(&path).unwrap();
            let inode = fs::metadata(&path).unwrap().ino();
            let mut desired = before.clone();
            desired.insert("ARTISTSORT".into(), vec!["Bowie, David".into()]);
            clean(&mut desired, "The XX", None, &allowed);
            assert_eq!(desired["ARTISTSORT"], ["Bowie, David"]);
            assert!(update(&path, &before, &desired).unwrap(), "{ext}");
            assert_eq!(
                fs::metadata(&path).unwrap().modified().unwrap(),
                stamp,
                "{ext}"
            );
            assert_eq!(super::super::pcm_hash(&path).unwrap(), hash, "{ext}");
            assert_eq!(
                fs::metadata(&path).unwrap().ino(),
                inode,
                "{ext}: file was replaced"
            );
            let tags = tagging::read(&path).unwrap().0;
            assert_eq!(tags["CUSTOM_VENDOR"], ["Keep This Value"], "{ext}");
            assert_eq!(tags["GENRE"], ["EBM", "Electronic"], "{ext}");
            assert_eq!(tags["TRACKARTIST"], ["The XX"], "{ext}");
            assert!(tagging::pictures(&path).unwrap().is_empty());
            assert!(!update(&path, &tags, &tags).unwrap());
        }
    }
    #[test]
    fn collection_enriches_offline_records_history_and_skips_completed_folders() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        let history = dir.path().join("history");
        fs::create_dir_all(&music).unwrap();
        let library_path = dir.path().join("library.sqlite3");
        library(&library_path);
        let path = music.join("keep.flac");
        media(&path, "Romy", "Proof");
        let mut source = tagging::read(&path).unwrap().0;
        source.insert("CATALOGNO".into(), vec!["CAT01".into()]);
        tagging::write(&path, &source).unwrap();
        let stamp = fs::metadata(&path).unwrap().modified().unwrap();
        let args = Args::try_parse_from([
            "bcfunl",
            "--collection",
            "--serverdir",
            music.to_str().unwrap(),
            "--output",
            history.to_str().unwrap(),
            "--library",
            library_path.to_str().unwrap(),
            "--parallel",
            "3",
        ])
        .unwrap();
        args.validate().unwrap();
        run(&args).unwrap();
        let tags = tagging::read(&path).unwrap().0;
        assert_eq!(tags["DISCOGSID"], ["1"]);
        assert_eq!(tags["CATALOGNO"], ["CAT01"]);
        assert_eq!(tags["RELEASECOUNTRY"], ["GB"]);
        assert_eq!(tags["GENRE"], ["Electronic", "Ambient"]);
        assert_eq!(tags["DATE"], ["2020-02-03"]);
        assert_eq!(tags["YEAR"], ["2020"]);
        assert_eq!(tags["ALBUMARTIST"], ["Romy"]);
        assert_eq!(tags["TRACKARTIST"], ["Guest"]);
        assert_eq!(tags["COMPOSER"], ["Composer One"]);
        assert_eq!(tags["CONDUCTOR"], ["Conductor One"]);
        assert!(tags["PERFORMER"].contains(&"Track Player".into()));
        assert_eq!(tags["CUSTOM_VENDOR"], ["Keep This Value"]);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), stamp);
        let db = Database::read_only(history.join("collection.sqlite3").to_str().unwrap()).unwrap();
        assert_eq!(
            db.query("SELECT artist,title FROM collection", &[])
                .unwrap()[0],
            [Some("Romy".into()), Some("Proof".into())]
        );
        let before = fs::read(&path).unwrap();
        run(&args).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!music.join("cover.jpg").exists());
    }
    #[test]
    fn multiple_releases_run_concurrently_and_keep_all_embedded_pictures() {
        use lofty::{config::WriteOptions, file::TaggedFileExt, prelude::TagExt};
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        let history = dir.path().join("history");
        fs::create_dir(&music).unwrap();
        let library_path = dir.path().join("library.sqlite3");
        library(&library_path);
        let seed = dir.path().join("seed.flac");
        media(&seed, "Romy", "Proof");
        let mut file = lofty::read_from_path(&seed).unwrap();
        let picture =
            lofty::picture::Picture::from_reader(&mut File::open("assets/bcfunl.png").unwrap())
                .unwrap();
        let tag = file.primary_tag_mut().unwrap();
        let mut front = picture.clone();
        front.set_pic_type(lofty::picture::PictureType::CoverFront);
        tag.push_picture(front);
        let mut back = picture;
        back.set_pic_type(lofty::picture::PictureType::CoverBack);
        tag.push_picture(back);
        tag.save_to_path(&seed, WriteOptions::default()).unwrap();
        let pictures = tagging::pictures(&seed).unwrap();
        assert_eq!(pictures.len(), 2);
        let mut stamps = Vec::new();
        for n in 0..4 {
            let folder = music.join(format!("release{n}"));
            fs::create_dir(&folder).unwrap();
            let path = folder.join("original.flac");
            fs::copy(&seed, &path).unwrap();
            stamps.push((
                path.clone(),
                fs::metadata(&path).unwrap().modified().unwrap(),
            ));
        }
        let args = Args::try_parse_from([
            "bcfunl",
            "--collection",
            "--serverdir",
            music.to_str().unwrap(),
            "--output",
            history.to_str().unwrap(),
            "--library",
            library_path.to_str().unwrap(),
            "--parallel",
            "16",
        ])
        .unwrap();
        run(&args).unwrap();
        for (path, stamp) in stamps {
            assert_eq!(tagging::pictures(&path).unwrap(), pictures);
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), stamp);
        }
        let db = Database::read_only(history.join("collection.sqlite3").to_str().unwrap()).unwrap();
        assert_eq!(
            db.query("SELECT count(*) FROM collection", &[]).unwrap()[0][0].as_deref(),
            Some("4")
        );
    }
    #[test]
    fn collection_matching_respects_existing_ids_catalogs_and_known_countries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite3");
        library(&path);
        let db = Database::read_only(path.to_str().unwrap()).unwrap();
        let info = Release {
            key: String::new(),
            artist: "Romy".into(),
            album: "Proof".into(),
            source: PathBuf::new(),
            genre: String::new(),
            country: String::new(),
            catalog: String::new(),
            publish: String::new(),
            url: String::new(),
        };
        for (id, cat, country, expected) in [
            ("", "", "", false),
            ("1", "cat-01", "GB", true),
            ("2", "", "", false),
            ("", "WRONG", "", false),
            ("", "CAT01", "US", false),
        ] {
            let mut tags = Tags::from([
                ("ARTIST".into(), vec!["Romy".into()]),
                ("ALBUM".into(), vec!["Proof".into()]),
            ]);
            for (key, v) in [
                ("DISCOGSID", id),
                ("CATALOGNO", cat),
                ("RELEASECOUNTRY", country),
            ] {
                if !v.is_empty() {
                    tags.insert(key.into(), vec![v.into()]);
                }
            }
            let track = Track {
                path: PathBuf::new(),
                tags,
                duration: 1.,
            };
            assert_eq!(
                discogs::find_collection(&db, &info, &[track])
                    .unwrap()
                    .is_some(),
                expected,
                "{id} / {cat} / {country}"
            );
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn direct_update_preserves_extended_attributes_and_removes_all_unlisted_genres() {
        use std::os::fd::AsRawFd;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("attributes.flac");
        media(&path, "Romy", "Proof");
        let original = tagging::read(&path).unwrap().0;
        let mut desired = original.clone();
        desired.remove("GENRE");
        let name = c"user.bcfunl-test";
        let file = File::open(&path).unwrap();
        assert_eq!(
            unsafe {
                libc::fsetxattr(
                    file.as_raw_fd(),
                    name.as_ptr(),
                    b"retained".as_ptr().cast(),
                    8,
                    0,
                )
            },
            0
        );
        drop(file);
        update(&path, &original, &desired).unwrap();
        assert!(!tagging::read(&path).unwrap().0.contains_key("GENRE"));
        let file = File::open(&path).unwrap();
        let mut buffer = [0u8; 8];
        assert_eq!(
            unsafe {
                libc::fgetxattr(
                    file.as_raw_fd(),
                    name.as_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            },
            8
        );
        assert_eq!(&buffer, b"retained");
    }
    #[test]
    fn canonicalizes_stored_delimiters_and_refuses_stale_tag_edits() {
        use lofty::{
            config::WriteOptions,
            file::TaggedFileExt,
            prelude::{ItemKey, TagExt},
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("delimited.flac");
        media(&path, "Romy", "Proof");
        let mut file = lofty::read_from_path(&path).unwrap();
        let tag = file.primary_tag_mut().unwrap();
        tag.remove_key(ItemKey::Genre);
        tag.insert_text(ItemKey::Genre, "EBM;Electronic".into());
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let before = tagging::read(&path).unwrap().0;
        assert_eq!(before["GENRE"], ["EBM", "Electronic"]);
        assert!(update(&path, &before, &before).unwrap());
        assert_eq!(
            tagging::raw_tags(&path).unwrap()["GENRE"],
            ["EBM", "Electronic"]
        );
        let mut desired = before.clone();
        desired.remove("GENRE");
        let mut external = before.clone();
        external.insert(
            "CUSTOM_VENDOR".into(),
            vec!["Changed by another writer".into()],
        );
        tagging::write(&path, &external).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(
            update(&path, &before, &desired)
                .unwrap_err()
                .to_string()
                .contains("changed before")
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn folder_metadata_is_tagged_without_a_match_and_master_does_not_invent_pressings() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let library_path = dir.path().join("library.sqlite3");
        library(&library_path);
        let db = Database::connect(library_path.to_str().unwrap()).unwrap();
        db.query(
            "CREATE TABLE masters(id INTEGER PRIMARY KEY,title TEXT,genres TEXT,styles TEXT)",
            &[],
        )
        .unwrap();
        db.query(
            "CREATE TABLE master_artists(master_id INTEGER,position INTEGER,name TEXT,anv TEXT)",
            &[],
        )
        .unwrap();
        db.query(
            r#"INSERT INTO masters VALUES(9,'The Hurting','["Electronic"]','["Ambient"]')"#,
            &[],
        )
        .unwrap();
        db.query(
            "INSERT INTO master_artists VALUES(9,0,'Tears For Fears',NULL)",
            &[],
        )
        .unwrap();
        drop(db);
        let folder = music.join("The Hurting [Super Deluxe Edition] [Live Sessions]");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("original.flac");
        media(
            &path,
            "Tears For Fears",
            "The Hurting [Super Deluxe Edition] [Live Sessions]",
        );
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        let walked = release(std::slice::from_ref(&path), &library_path).unwrap();
        assert_eq!(walked.matched, 1);
        let tags = tagging::read(&path).unwrap().0;
        assert_eq!(tags["DISCOGSMASTERID"], ["9"]);
        assert_eq!(tags["GENRE"], ["Electronic", "Ambient"]);
        for key in [
            "DISCOGSID",
            "CATALOGNUMBER",
            "CATALOGNO",
            "RELEASECOUNTRY",
            "COUNTRY_ISO",
            "LABEL",
            "DATE",
            "YEAR",
        ] {
            assert!(!tags.contains_key(key), "{key} was invented");
        }
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        let again = release(std::slice::from_ref(&path), &library_path).unwrap();
        assert_eq!(again.matched, 1);
        assert_eq!(again.changed, 0);
        let folder = music.join("Woman In Chains [EU] [876 249-2]");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("same.flac");
        media(&path, "Tears For Fears", "Woman In Chains [Eu] [876 249-2]");
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(
            release(std::slice::from_ref(&path), &library_path)
                .unwrap()
                .matched,
            0
        );
        let tags = tagging::read(&path).unwrap().0;
        assert_eq!(tags["ALBUM"], ["Woman In Chains"]);
        assert_eq!(tags["COUNTRY_ISO"], ["EU"]);
        assert_eq!(tags["RELEASECOUNTRY"], ["EU"]);
        assert_eq!(tags["CATALOGNUMBER"], ["876 249-2"]);
        assert_eq!(tags["CATALOGNO"], ["876 249-2"]);
        assert!(!tags.contains_key("DISCOGSID"));
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        let db = Database::connect(library_path.to_str().unwrap()).unwrap();
        let body = json!({"title":"Woman In Chains","artists":[{"name":"Tears For Fears"}],"country":"Europe","labels":[{"name":"Fixture Label","catno":"8762492"}],"genres":["Electronic"],"styles":["Ambient"]});
        db.query(
            "INSERT INTO releases VALUES(2,'woman in chains',?)",
            &[&body.to_string()],
        )
        .unwrap();
        drop(db);
        assert_eq!(
            release(std::slice::from_ref(&path), &library_path)
                .unwrap()
                .matched,
            1
        );
        let tags = tagging::read(&path).unwrap().0;
        assert_eq!(tags["DISCOGSID"], ["2"]);
        assert_eq!(tags["CATALOGNUMBER"], ["876 249-2"]);
        assert_eq!(tags["COUNTRY_ISO"], ["EU"]);
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), before);
    }
    #[test]
    fn folder_hints_follow_disc_parent_and_preserve_existing_facts() {
        let mut tags = Tags::from([
            (
                "ALBUM".into(),
                vec!["The Seeds Of Love [UK] [558 105-2]".into()],
            ),
            ("ARTIST".into(), vec!["Tears For Fears".into()]),
        ]);
        folder_hints(
            &mut tags,
            Path::new("/music/The Seeds Of Love [UK] [558 105-2]/CD1/01.flac"),
        );
        assert_eq!(tags["ALBUM"], ["The Seeds Of Love"]);
        assert_eq!(tags["COUNTRY_ISO"], ["GB"]);
        assert_eq!(tags["CATALOGNUMBER"], ["558 105-2"]);
        tags.insert("RELEASECOUNTRY".into(), vec!["US".into()]);
        tags.insert("CATALOGNUMBER".into(), vec!["KNOWN01".into()]);
        folder_hints(
            &mut tags,
            Path::new("/music/The Seeds Of Love [UK] [558 105-2]/01.flac"),
        );
        assert_eq!(tags["COUNTRY_ISO"], ["US"]);
        assert_eq!(tags["CATALOGNUMBER"], ["KNOWN01"]);
    }
    #[test]
    fn failed_metadata_edit_restores_original_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timestamps");
        fs::write(&path, b"original").unwrap();
        let file = File::open(&path).unwrap();
        let before = file.metadata().unwrap();
        let result = preserve_times(&file, &before, || {
            file.set_modified(std::time::UNIX_EPOCH)?;
            anyhow::bail!("Injected tag failure")
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Injected tag failure")
        );
        assert_eq!(
            file.metadata().unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
    }
    #[test]
    fn metadata_walk_does_not_decode_the_audio_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("damaged-audio.flac");
        media(&path, "Romy", "Proof");
        let mut bytes = fs::read(&path).unwrap();
        let mut offset = 4;
        loop {
            let last = bytes[offset] & 128 != 0;
            let length = (usize::from(bytes[offset + 1]) << 16)
                | (usize::from(bytes[offset + 2]) << 8)
                | usize::from(bytes[offset + 3]);
            offset += 4 + length;
            if last {
                break;
            }
        }
        bytes[offset..offset + 4].fill(0);
        fs::write(&path, bytes).unwrap();
        assert!(super::super::pcm_hash(&path).is_err());
        let original = tagging::read(&path).unwrap().0;
        let mut desired = original.clone();
        desired.insert("GENRE".into(), vec!["Ambient".into()]);
        assert!(update(&path, &original, &desired).unwrap());
        assert_eq!(tagging::read(&path).unwrap().0["GENRE"], ["Ambient"]);
        assert!(!fs::read_dir(dir.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".bcfunl-process-")
        }));
    }
    #[test]
    fn date_window_defaults_to_calendar_months_and_has_a_strict_utc_boundary() {
        let args =
            Args::try_parse_from(["bcfunl", "--collection", "--serverdir", "music"]).unwrap();
        for (today, expected) in [
            ("2026-11-15", "2026-09-15"),
            ("2024-04-30", "2024-02-29"),
            ("2026-01-31", "2025-11-30"),
        ] {
            let day = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap();
            assert_eq!(cutoff(&args, day).unwrap().to_string(), expected);
        }
        let from = chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        for (stamp, expected) in [
            ("2026-09-14T23:59:59Z", false),
            ("2026-09-15T23:59:59Z", false),
            ("2026-09-16T00:00:00Z", true),
        ] {
            assert_eq!(
                modified_after(
                    chrono::DateTime::parse_from_rfc3339(stamp)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    from
                ),
                expected
            );
        }
        let explicit = Args::try_parse_from([
            "bcfunl",
            "--collection",
            "--serverdir",
            "music",
            "--fromdt",
            "2000-01-01",
        ])
        .unwrap();
        explicit.validate().unwrap();
        assert_eq!(cutoff(&explicit, from).unwrap().to_string(), "2000-01-01");
    }
    #[test]
    fn date_window_is_applied_before_count_and_force_cannot_override_it() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        let history = dir.path().join("history");
        let old = music.join("A Old");
        let recent = music.join("Z Recent");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir(&recent).unwrap();
        let old_path = old.join("invalid.flac");
        fs::write(&old_path, b"not an audio container").unwrap();
        File::open(&old_path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1000))
            .unwrap();
        let recent_path = recent.join("one.flac");
        media(&recent_path, "Romy", "Proof");
        let library_path = dir.path().join("library.sqlite3");
        library(&library_path);
        let mut args = Args::try_parse_from([
            "bcfunl",
            "--collection",
            "--force",
            "--serverdir",
            music.to_str().unwrap(),
            "--output",
            history.to_str().unwrap(),
            "--library",
            library_path.to_str().unwrap(),
            "--fromdt",
            "2000-01-01",
            "--count",
            "1",
        ])
        .unwrap();
        run(&args).unwrap();
        let db = Database::connect(history.join("collection.sqlite3").to_str().unwrap()).unwrap();
        assert_eq!(
            db.query("SELECT path FROM collection", &[]).unwrap(),
            vec![vec![Some(recent.to_str().unwrap().into())]]
        );
        assert_eq!(fs::read(&old_path).unwrap(), b"not an audio container");
        let latest = chrono::DateTime::<chrono::Utc>::from(
            fs::metadata(&recent_path).unwrap().modified().unwrap(),
        );
        assert!(already_walked(&db, &recent, latest).unwrap());
        db.query(
            "UPDATE collection SET walk_date='2001-01-01T00:00:00Z'",
            &[],
        )
        .unwrap();
        assert!(!already_walked(&db, &recent, latest).unwrap());
        args.force = false;
        run(&args).unwrap();
        assert!(already_walked(&db, &recent, latest).unwrap());
    }
    #[test]
    fn refuses_invalid_audio_or_hardlinks_without_touching_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.flac");
        fs::write(&path, b"not audio").unwrap();
        let tags = Tags::from([("GENRE".into(), vec!["Ambient".into()])]);
        assert!(update(&path, &Tags::new(), &tags).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not audio");
        fs::hard_link(&path, dir.path().join("link.flac")).unwrap();
        assert!(
            update(&path, &Tags::new(), &tags)
                .unwrap_err()
                .to_string()
                .contains("hard-linked")
        );
    }
}
