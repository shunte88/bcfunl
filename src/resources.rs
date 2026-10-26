//! Discogs monthly artist identity snapshots, separate from collection history.
use crate::{Args, db::Database, processing::tempfile, shutdown};
use anyhow::{Context, Result, bail, ensure};
use chrono::{Datelike, NaiveDate, Utc};
use quick_xml::{Reader, events::Event};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

const SITE: &str = "https://data.discogs.com/";
fn dump_date(name: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(
        name.strip_prefix("discogs_")?
            .strip_suffix("_artists.xml.gz")?,
        "%Y%m%d",
    )
    .ok()
}
fn latest_kind(html: &str, today: NaiveDate, kind: &str) -> Result<String> {
    let doc = Html::parse_document(html);
    doc.select(&Selector::parse("a[href]").unwrap()).filter_map(|e| {
        let url = reqwest::Url::parse(SITE).ok()?.join(e.value().attr("href")?).ok()?;
        if url.origin() != reqwest::Url::parse(SITE).ok()?.origin() { return None; }
        let key = url.query_pairs().find(|(k, _)| k == "download")?.1.into_owned();
        let name = key.strip_prefix(&format!("data/{}/", today.year()))?;
        let artist_name = name.strip_suffix(&format!("_{kind}.xml.gz"))?.to_owned() + "_artists.xml.gz";
            let date = dump_date(&artist_name)?;
        (date.year() == today.year() && date.month() == today.month() && date <= today).then(|| name.to_owned())
    }).max().with_context(|| format!("No Discogs {kind} dump is published for the current UTC month; use a local snapshot override"))
}
#[cfg(test)]
fn latest(html: &str, today: NaiveDate) -> Result<String> {
    latest_kind(html, today, "artists")
}
fn download_url(year: i32, name: &str) -> String {
    let mut url = reqwest::Url::parse(SITE).unwrap();
    url.query_pairs_mut()
        .append_pair("download", &format!("data/{year}/{name}"));
    url.into()
}
fn sha256(path: &Path) -> Result<String> {
    shutdown::check()?;
    let out = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .context("Run sha256sum")?;
    ensure!(out.status.success(), "sha256sum failed");
    let digest = String::from_utf8(out.stdout)?
        .split_whitespace()
        .next()
        .context("Missing SHA-256")?
        .to_owned();
    ensure!(
        digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid SHA-256"
    );
    Ok(digest)
}
fn expected_checksum(text: &str, name: &str) -> Result<String> {
    text.lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let digest = fields.next()?;
            let file = fields.next()?.trim_start_matches('*');
            (file == name && digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_hexdigit()))
                .then(|| digest.to_ascii_lowercase())
        })
        .context("Resource dump is missing from Discogs SHA-256 manifest")
}
async fn acquire(root: &Path, temporary: &Path, kind: &str) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("bcfunl/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(7200))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() >= 5
                || url.scheme() != "https"
                || url.port_or_known_default() != Some(443)
                || !matches!(
                    url.host_str(),
                    Some(
                        "data.discogs.com"
                            | "discogs-data-dumps.s3.us-west-2.amazonaws.com"
                            | "discogs-data-dumps.s3-us-west-2.amazonaws.com"
                    )
                )
            {
                attempt.error("Unexpected Discogs resource redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let today = Utc::now().date_naive();
    let listing = client
        .get(SITE)
        .query(&[("prefix", format!("data/{}/", today.year()))])
        .send()
        .await
        .map_err(|e| e.without_url())?
        .error_for_status()
        .map_err(|e| e.without_url())?
        .text()
        .await?;
    let name = latest_kind(&listing, today, kind)?;
    let checksum_name = name.replace(&format!("_{kind}.xml.gz"), "_CHECKSUM.txt");
    let manifest = client
        .get(download_url(today.year(), &checksum_name))
        .send()
        .await
        .map_err(|e| e.without_url())?
        .error_for_status()
        .map_err(|e| e.without_url())?
        .text()
        .await?;
    let expected = expected_checksum(&manifest, &name)?;
    let target = root.join(&name);
    if target.is_file() && sha256(&target)? == expected {
        eprintln!("Verified cached {name}");
        return Ok(target);
    }
    eprintln!("Downloading {name}");
    let staged = temporary.join(&name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)?;
    let mut response = client
        .get(download_url(today.year(), &name))
        .send()
        .await
        .map_err(|e| e.without_url())?
        .error_for_status()
        .map_err(|e| e.without_url())?;
    let expected_bytes = response.content_length();
    let mut bytes = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.without_url())? {
        shutdown::check()?;
        file.write_all(&chunk)?;
        bytes += chunk.len() as u64;
    }
    ensure!(
        expected_bytes.is_none_or(|n| bytes == n),
        "Incomplete Discogs download"
    );
    file.sync_all()?;
    drop(file);
    ensure!(
        sha256(&staged)? == expected,
        "Discogs artist checksum mismatch"
    );
    fs::rename(&staged, &target)?;
    File::open(root)?.sync_all()?;
    Ok(target)
}

#[derive(Default)]
struct Frame {
    name: String,
    text: String,
    attributes: serde_json::Map<String, Value>,
    children: Vec<Frame>,
}
impl Frame {
    fn text(&self, name: &str) -> &str {
        self.children
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.text.as_str())
            .unwrap_or("")
    }
    fn items(&self, name: &str) -> &[Frame] {
        self.children
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.children.as_slice())
            .unwrap_or_default()
    }
}
fn schema(db: &Database) -> Result<()> {
    for sql in [
        "CREATE TABLE artists (id INTEGER PRIMARY KEY CHECK(id>0),name TEXT NOT NULL,real_name TEXT,profile TEXT,data_quality TEXT,urls TEXT NOT NULL,name_variations TEXT NOT NULL,aliases TEXT NOT NULL,members TEXT NOT NULL,groups TEXT NOT NULL,genres TEXT,styles TEXT)",
        "CREATE TABLE masters(id INTEGER PRIMARY KEY CHECK(id>0),title TEXT NOT NULL,main_release_id INTEGER,year INTEGER,data_quality TEXT,genres TEXT NOT NULL,styles TEXT NOT NULL)",
        "CREATE TABLE master_artists(master_id INTEGER NOT NULL,artist_id INTEGER,position INTEGER NOT NULL,name TEXT NOT NULL,anv TEXT,join_text TEXT,PRIMARY KEY(master_id,position))",
        "CREATE TABLE artist_genres(artist_id INTEGER NOT NULL,genre TEXT COLLATE NOCASE NOT NULL,master_count INTEGER NOT NULL,PRIMARY KEY(artist_id,genre)) WITHOUT ROWID",
        "CREATE TABLE resource_import(dataset TEXT PRIMARY KEY,source_file TEXT NOT NULL,source_sha256 TEXT NOT NULL,imported_at TEXT NOT NULL,record_count INTEGER NOT NULL,field_counts TEXT NOT NULL)",
    ] {
        db.query(sql, &[])?;
    }
    Ok(())
}
fn save_artist(db: &Database, artist: &Frame) -> Result<()> {
    let list = |key: &str| {
        let values: Vec<_> = artist
            .items(key)
            .iter()
            .map(|entry| {
                if entry.attributes.is_empty() && !matches!(key, "aliases" | "members" | "groups") {
                    Value::String(entry.text.clone())
                } else {
                    let mut object = entry.attributes.clone();
                    object.insert(
                        if entry.name == "id" { "id" } else { "name" }.into(),
                        Value::String(entry.text.clone()),
                    );
                    Value::Object(object)
                }
            })
            .collect();
        json!(values).to_string()
    };
    let id = artist.text("id");
    ensure!(
        id.parse::<u64>().is_ok_and(|n| n > 0),
        "Missing or invalid Discogs artist ID"
    );
    // Removed Discogs records may lack a name; retain the ID as source evidence.
    db.query("INSERT INTO artists(id,name,real_name,profile,data_quality,urls,name_variations,aliases,members,groups) VALUES(?,?,NULLIF(?,''),NULLIF(?,''),NULLIF(?,''),?,?,?,?,?)", &[id,artist.text("name"),artist.text("realname"),artist.text("profile"),artist.text("data_quality"),&list("urls"),&list("namevariations"),&list("aliases"),&list("members"),&list("groups")])?;
    Ok(())
}
fn save_master(db: &Database, master: &Frame, map: &vcw_metadata::Genres) -> Result<()> {
    let id = master
        .attributes
        .get("id")
        .and_then(Value::as_str)
        .context("Missing master ID")?;
    ensure!(id.parse::<u64>().is_ok_and(|n| n > 0), "Invalid master ID");
    let genres: Vec<_> = master
        .items("genres")
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    let styles: Vec<_> = master
        .items("styles")
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    db.query(
        "INSERT INTO masters VALUES(?,?,NULLIF(?,''),NULLIF(NULLIF(?,''),'0'),NULLIF(?,''),?,?)",
        &[
            id,
            master.text("title"),
            master.text("main_release"),
            master.text("year"),
            master.text("data_quality"),
            &json!(genres).to_string(),
            &json!(styles).to_string(),
        ],
    )?;
    let normalized = map.normalize_all(genres.iter().chain(styles.iter()));
    let mut seen = BTreeSet::new();
    for (position, artist) in master.items("artists").iter().enumerate() {
        let artist_id = artist.text("id");
        let known_id = artist_id.parse::<u64>().is_ok_and(|n| n > 0);
        db.query(
            "INSERT INTO master_artists VALUES(?,NULLIF(?,''),?,?,NULLIF(?,''),NULLIF(?,''))",
            &[
                id,
                if known_id { artist_id } else { "" },
                &position.to_string(),
                artist.text("name"),
                artist.text("anv"),
                artist.text("join"),
            ],
        )?;
        // Keep unlinked source credits without inventing artist-genre associations.
        if known_id && seen.insert(artist_id) {
            for genre in &normalized {
                db.query("INSERT INTO artist_genres VALUES(?,?,1) ON CONFLICT(artist_id,genre) DO UPDATE SET master_count=master_count+1",&[artist_id,genre])?;
            }
        }
    }
    Ok(())
}
fn stream(
    reader: impl BufRead,
    root: &str,
    record: &str,
    mut save: impl FnMut(&Frame) -> Result<()>,
) -> Result<(u64, BTreeMap<String, u64>)> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().expand_empty_elements = true;
    let mut buffer = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut count = 0u64;
    let mut fields = BTreeMap::new();
    let mut root_seen = false;
    let mut root_closed = false;
    loop {
        shutdown::check()?;
        let finished = match xml
            .read_event_into(&mut buffer)
            .context("Read Discogs XML")?
        {
            Event::DocType(_) => bail!("XML document types are not supported"),
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())?.to_owned();
                if stack.is_empty() {
                    ensure!(name == root && !root_seen, "Expected one <{root}> root");
                    root_seen = true;
                }
                if stack.len() == 1 {
                    ensure!(name == record, "Unexpected element under {root}");
                }
                let mut frame = Frame {
                    name,
                    ..Frame::default()
                };
                for attr in e.attributes() {
                    let attr = attr?;
                    frame.attributes.insert(
                        std::str::from_utf8(attr.key.as_ref())?.to_owned(),
                        Value::String(attr.decode_and_unescape_value(xml.decoder())?.into_owned()),
                    );
                }
                stack.push(frame);
                ensure!(stack.len() <= 32, "Discogs XML nesting too deep");
                None
            }
            Event::Text(e) => {
                if stack.len() == 1 {
                    ensure!(
                        e.unescape()?.trim().is_empty(),
                        "Text between Discogs records"
                    );
                } else if let Some(frame) = stack.last_mut() {
                    frame.text.push_str(&e.unescape()?);
                    ensure!(
                        frame.text.len() <= 8 * 1024 * 1024,
                        "Discogs field too large"
                    );
                } else {
                    ensure!(e.unescape()?.trim().is_empty(), "Text outside XML root");
                }
                None
            }
            Event::CData(e) => {
                if let Some(frame) = stack.last_mut() {
                    frame.text.push_str(std::str::from_utf8(e.as_ref())?);
                    ensure!(
                        frame.text.len() <= 8 * 1024 * 1024,
                        "Discogs field too large"
                    );
                }
                None
            }
            Event::End(_) => Some(stack.pop().context("Unexpected closing XML element")?),
            Event::Eof => break,
            _ => None,
        };
        if let Some(frame) = finished {
            if stack.len() == 1 {
                ensure!(frame.name == record, "Unexpected record end");
                for field in &frame.children {
                    *fields.entry(field.name.clone()).or_default() += 1;
                }
                save(&frame)?;
                count += 1;
                if count.is_multiple_of(100_000) {
                    eprintln!("Imported {count} {root}");
                }
            } else if let Some(parent) = stack.last_mut() {
                parent.children.push(frame);
            } else {
                root_closed = true;
            }
        }
        buffer.clear();
    }
    ensure!(
        root_seen && root_closed && stack.is_empty() && count > 0,
        "Incomplete or empty Discogs {root} XML"
    );
    Ok((count, fields))
}
fn import(reader: impl BufRead, db: &Database) -> Result<(u64, BTreeMap<String, u64>)> {
    stream(reader, "artists", "artist", |a| save_artist(db, a))
}
fn import_source(
    path: &Path,
    db: &Database,
    masters: bool,
) -> Result<(u64, BTreeMap<String, u64>)> {
    let mut file = File::open(path).context("Open Discogs extract")?;
    let mut magic = [0; 2];
    let n = file.read(&mut magic)?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0))?;
    let map = vcw_metadata::Genres::builtin();
    let parse = |reader: Box<dyn BufRead>| {
        if masters {
            stream(reader, "masters", "master", |m| save_master(db, m, &map))
        } else {
            import(reader, db)
        }
    };
    if n == 2 && magic == [0x1f, 0x8b] {
        let mut child = crate::aria2::ChildGuard(
            Command::new("gzip")
                .args(["-cd", "--"])
                .arg(path)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .context("Start gzip")?,
        );
        let result = parse(Box::new(BufReader::new(
            child.0.stdout.take().context("gzip stdout")?,
        )))?;
        ensure!(
            child.0.wait()?.success(),
            "Invalid or truncated gzip extract"
        );
        Ok(result)
    } else {
        parse(Box::new(BufReader::new(file)))
    }
}

fn build(
    source: &Path,
    masters: &Path,
    target: &Path,
    temporary: &Path,
    force: bool,
) -> Result<(u64, u64)> {
    let hash = sha256(source)?;
    let master_hash = sha256(masters)?;
    if target.is_file() && !force {
        let db = Database::connect(target.to_str().context("Library path is not UTF-8")?)?;
        let rows=db.query("SELECT record_count FROM resource_import WHERE dataset='artists' AND source_sha256=? AND EXISTS(SELECT 1 FROM resource_import WHERE dataset='masters' AND source_sha256=?)",&[&hash,&master_hash])?;
        if let Some(row) = rows.first() {
            eprintln!("Library already contains these snapshots");
            let n = db.query(
                "SELECT record_count FROM resource_import WHERE dataset='masters'",
                &[],
            )?;
            return Ok((
                row[0].as_deref().unwrap_or("0").parse()?,
                n[0][0].as_deref().unwrap_or("0").parse()?,
            ));
        }
    }
    let staged = temporary.join("library.sqlite3");
    let db = Database::connect(staged.to_str().context("Library path is not UTF-8")?)?;
    db.query("PRAGMA journal_mode=DELETE", &[])?;
    db.query("PRAGMA synchronous=FULL", &[])?;
    db.query("PRAGMA cache_size=-65536", &[])?;
    schema(&db)?;
    db.query("BEGIN IMMEDIATE", &[])?;
    let mut counts = BTreeMap::new();
    for (kind, path, digest) in [
        ("masters", masters, &master_hash),
        ("artists", source, &hash),
    ] {
        eprintln!("Importing {kind} from {}", path.display());
        let (count, fields) = import_source(path, &db, kind == "masters")?;
        db.query(
            "INSERT INTO resource_import VALUES(?,?,?,?,?,?)",
            &[
                kind,
                &path.file_name().unwrap_or_default().to_string_lossy(),
                digest,
                &Utc::now().to_rfc3339(),
                &count.to_string(),
                &json!(fields).to_string(),
            ],
        )?;
        counts.insert(kind, count);
    }
    eprintln!("Building artist genre defaults and indexes");
    db.query("UPDATE artists SET genres=(SELECT json_group_array(genre) FROM (SELECT genre FROM artist_genres WHERE artist_id=artists.id ORDER BY master_count DESC,genre COLLATE NOCASE)) WHERE EXISTS(SELECT 1 FROM artist_genres WHERE artist_id=artists.id)",&[])?;
    db.query("COMMIT", &[])?;
    for sql in [
        "CREATE INDEX artists_name ON artists(name COLLATE NOCASE)",
        "CREATE INDEX artists_real_name ON artists(real_name COLLATE NOCASE) WHERE real_name IS NOT NULL",
        "CREATE INDEX master_artist_lookup ON master_artists(artist_id,master_id)",
        "CREATE INDEX masters_title ON masters(title COLLATE NOCASE)",
    ] {
        db.query(sql, &[])?;
    }
    ensure!(
        db.query("PRAGMA quick_check", &[])? == vec![vec![Some("ok".into())]],
        "Library integrity check failed"
    );
    drop(db);
    shutdown::check()?;
    File::open(&staged)?.sync_all()?;
    fs::rename(&staged, target)?;
    File::open(target.parent().unwrap())?.sync_all()?;
    Ok((counts["artists"], counts["masters"]))
}
pub fn run(args: &Args) -> Result<()> {
    let root = args.output.join("resources");
    fs::create_dir_all(&root).context("Create resources directory")?;
    let lock = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(root.join(".resources.lock"))?;
    lock.try_lock()
        .context("Another resource import is running")?;
    let temporary = tempfile::tempdir_in(&root)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let source = match &args.resource_file {
        Some(p) => p.clone(),
        None => {
            runtime.block_on(shutdown::until(acquire(&root, temporary.path(), "artists")))??
        }
    };
    let masters = match &args.master_file {
        Some(p) => p.clone(),
        None => {
            runtime.block_on(shutdown::until(acquire(&root, temporary.path(), "masters")))??
        }
    };
    let (artists, masters) = build(
        &source,
        &masters,
        &root.join("library.sqlite3"),
        temporary.path(),
        args.force,
    )?;
    eprintln!(
        "Library ready: {} ({artists} artists, {masters} masters)",
        root.join("library.sqlite3").display()
    );
    eprintln!(
        "Artist genres are master-derived fallback evidence, not recording-specific classifications."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    const MASTERS: &str = r#"<masters><master id="10"><main_release>100</main_release><artists><artist><id>1</id><name>A &amp; B</name></artist></artists><genres><genre>Electronic</genre></genres><styles><style>Acid House</style></styles><year>2026</year><title>Proof</title></master></masters>"#;
    const XML: &str = r#"<artists><artist><id>1</id><name>A &amp; B</name><realname>Real Name</realname><profile><![CDATA[Not a genre: jazz & folk]]></profile><data_quality>Correct</data_quality><urls><url>https://example.org</url></urls><namevariations><name>A and B</name></namevariations><aliases><name id="22">Alias</name></aliases><members><id>33</id><name id="44" active="false">Former</name></members><groups><name id="55">Group</name></groups></artist><artist><id>2</id><data_quality>Needs Vote</data_quality></artist></artists>"#;
    #[test]
    fn latest_month_and_checksums_use_only_official_expected_files() {
        let html = r#"<a href="?download=data%2F2026%2Fdiscogs_20261001_artists.xml.gz">one</a><a href="?download=data%2F2026%2Fdiscogs_20261015_artists.xml.gz">latest</a><a href="?download=data%2F2026%2Fdiscogs_20261031_artists.xml.gz">future</a><a href="?download=data%2F2026%2Fdiscogs_20261001_releases.xml.gz">release</a><a href="https://evil.org/?download=data%2F2026%2Fdiscogs_20261025_artists.xml.gz">bad</a>"#;
        assert_eq!(
            latest(html, NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()).unwrap(),
            "discogs_20261015_artists.xml.gz"
        );
        assert!(latest(html, NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()).is_err());
        let name = "discogs_20261001_artists.xml.gz";
        let digest = "a".repeat(64);
        assert_eq!(
            expected_checksum(&format!("{digest} {name}"), name).unwrap(),
            digest
        );
        assert!(expected_checksum("wrong other.xml.gz", name).is_err());
        let url = reqwest::Url::parse(&download_url(2026, name)).unwrap();
        assert_eq!(url.host_str(), Some("data.discogs.com"));
        assert_eq!(
            url.query_pairs().next().unwrap().1,
            format!("data/2026/{name}")
        );
    }
    #[test]
    fn imports_identity_and_relationships_without_inventing_genres() {
        let db = Database::connect(":memory:").unwrap();
        schema(&db).unwrap();
        let (count, fields) = import(XML.as_bytes(), &db).unwrap();
        assert_eq!(count, 2);
        assert_eq!(fields["id"], 2);
        let row=&db.query("SELECT name,real_name,profile,aliases,members,genres,styles FROM artists WHERE id=1",&[]).unwrap()[0];
        assert_eq!(row[0].as_deref(), Some("A & B"));
        assert_eq!(row[2].as_deref(), Some("Not a genre: jazz & folk"));
        let aliases: Value = serde_json::from_str(row[3].as_deref().unwrap()).unwrap();
        assert_eq!(aliases[0], json!({"id":"22","name":"Alias"}));
        let members: Value = serde_json::from_str(row[4].as_deref().unwrap()).unwrap();
        assert_eq!(members[0], json!({"id":"33"}));
        assert_eq!(members[1]["active"], "false");
        assert_eq!(row[5], None);
        assert_eq!(row[6], None);
        assert!(
            db.query("SELECT name FROM sqlite_master WHERE name='releases'", &[])
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn staged_rebuild_is_atomic_idempotent_and_checks_gzip_crc() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("artists.xml");
        let target = root.path().join("library.sqlite3");
        let masters = root.path().join("masters.xml");
        fs::write(&masters, MASTERS).unwrap();
        fs::write(&source, XML).unwrap();
        let work = tempfile::tempdir_in(root.path()).unwrap();
        assert_eq!(
            build(&source, &masters, &target, work.path(), false).unwrap(),
            (2, 1)
        );
        assert_eq!(
            build(&source, &masters, &target, work.path(), false).unwrap(),
            (2, 1)
        );
        let good = fs::read(&target).unwrap();
        fs::write(
            &source,
            "<artists><artist><id>3</id><name>Incomplete</name>",
        )
        .unwrap();
        assert!(build(&source, &masters, &target, work.path(), true).is_err());
        assert_eq!(fs::read(&target).unwrap(), good);
        for bad in [
            "<!DOCTYPE artists [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><artists/>",
            "<html>wrong download</html>",
            "<artists><artist><name>no id</name></artist></artists>",
            "<artists><artist><id>0</id></artist></artists>",
        ] {
            let db = Database::connect(":memory:").unwrap();
            schema(&db).unwrap();
            assert!(import(bad.as_bytes(), &db).is_err());
        }
        fs::write(&source, XML).unwrap();
        let gzip = Command::new("gzip")
            .arg("-c")
            .arg(&source)
            .output()
            .unwrap();
        assert!(gzip.status.success());
        let gz = root.path().join("artists.xml.gz");
        fs::write(&gz, &gzip.stdout).unwrap();
        let db = Database::connect(":memory:").unwrap();
        schema(&db).unwrap();
        assert_eq!(import_source(&gz, &db, false).unwrap().0, 2);
        let mut broken = gzip.stdout;
        let end = broken.len() - 5;
        broken[end] ^= 0xff;
        fs::write(&gz, broken).unwrap();
        let db = Database::connect(":memory:").unwrap();
        schema(&db).unwrap();
        assert!(import_source(&gz, &db, false).is_err());
    }
    #[test]
    fn resource_cli_requires_no_cookies_or_artist_map() {
        for options in [
            vec!["bcfunl", "--resources", "--process"],
            vec!["bcfunl", "--resources", "--artist", "A"],
            vec!["bcfunl", "--resource-file", "a.xml"],
        ] {
            assert!(Args::try_parse_from(options).is_err());
        }
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("artists.xml");
        fs::write(&source, XML).unwrap();
        let masters = root.path().join("masters.xml");
        fs::write(&masters, MASTERS).unwrap();
        let args = Args::try_parse_from([
            "bcfunl",
            "--resources",
            "--master-file",
            masters.to_str().unwrap(),
            "--resource-file",
            source.to_str().unwrap(),
            "--output",
            root.path().to_str().unwrap(),
            "--cookies",
            "missing-cookie-file",
        ])
        .unwrap();
        run(&args).unwrap();
        assert!(root.path().join("resources/library.sqlite3").is_file());
        assert!(!root.path().join("collection.sqlite3").exists());
    }
}

#[cfg(test)]
mod master_tests {
    use super::*;
    #[test]
    fn master_genres_are_normalized_ranked_and_counted_once_per_artist_per_master() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("artists.xml");
        let masters = root.path().join("masters.xml");
        let target = root.path().join("library.sqlite3");
        fs::write(&source,"<artists><artist><id>1</id><name>One</name></artist><artist><id>2</id><name>Two</name></artist><artist><id>3</id><name>No Evidence</name></artist></artists>").unwrap();
        fs::write(&masters,r#"<masters><master id="10"><title>Acid</title><artists><artist><id>1</id><name>One</name></artist><artist><id>1</id><name>One Again</name></artist></artists><genres><genre>Electronic</genre><genre>electronic</genre></genres><styles><style>Acid House</style></styles></master><master id="11"><title>Techno</title><artists><artist><id>1</id><name>One</name></artist><artist><id>2</id><name>Two</name></artist></artists><genres><genre>Electronic</genre></genres><styles><style>Techno</style></styles></master></masters>"#).unwrap();
        let work = tempfile::tempdir_in(root.path()).unwrap();
        assert_eq!(
            build(&source, &masters, &target, work.path(), false).unwrap(),
            (3, 2)
        );
        let db = Database::connect(target.to_str().unwrap()).unwrap();
        let rows=db.query("SELECT genre,master_count FROM artist_genres WHERE artist_id=1 ORDER BY master_count DESC,genre COLLATE NOCASE",&[]).unwrap();
        assert_eq!(rows[0], vec![Some("Electronic".into()), Some("2".into())]);
        assert!(rows.contains(&vec![Some("House".into()), Some("1".into())]));
        let genres: Value = serde_json::from_str(
            db.query("SELECT genres FROM artists WHERE id=1", &[])
                .unwrap()[0][0]
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(genres[0], "Electronic");
        assert_eq!(
            db.query("SELECT genres FROM artists WHERE id=3", &[])
                .unwrap()[0][0],
            None
        );
        assert_eq!(
            db.query("SELECT styles FROM masters WHERE id=10", &[])
                .unwrap()[0][0]
                .as_deref(),
            Some("[\"Acid House\"]")
        );
        assert_eq!(
            db.query(
                "SELECT count(*) FROM master_artists WHERE master_id=10",
                &[]
            )
            .unwrap()[0][0]
                .as_deref(),
            Some("2")
        );
        drop(db);
        let original = fs::read(&target).unwrap();
        fs::write(&masters, "<masters><master id='12'><title>broken").unwrap();
        assert!(build(&source, &masters, &target, work.path(), true).is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    #[test]
    fn root_whitespace_is_not_accumulated_as_an_artist_field() {
        let xml = format!(
            "<artists><artist><id>1</id></artist>{}<artist><id>2</id></artist></artists>",
            "\n".repeat(8 * 1024 * 1024 + 1)
        );
        let (count, _) = stream(xml.as_bytes(), "artists", "artist", |_| Ok(())).unwrap();
        assert_eq!(count, 2);
    }
}

#[cfg(test)]
mod incomplete_credit_tests {
    use super::*;
    #[test]
    fn nameless_id_links_are_retained_but_do_not_create_artist_genre_evidence() {
        let db = Database::connect(":memory:").unwrap();
        schema(&db).unwrap();
        let map = vcw_metadata::Genres::builtin();
        let xml = r#"<masters><master id="35849"><title>Seven Veils Of Silence</title><artists><artist><name>Hecate</name></artist><artist><id>0</id><name>Unknown</name></artist></artists><genres><genre>Electronic</genre></genres></master></masters>"#;
        assert_eq!(
            stream(xml.as_bytes(), "masters", "master", |m| save_master(
                &db, m, &map
            ))
            .unwrap()
            .0,
            1
        );
        let rows = db
            .query(
                "SELECT artist_id,name FROM master_artists ORDER BY position",
                &[],
            )
            .unwrap();
        assert_eq!(
            rows,
            vec![
                vec![None, Some("Hecate".into())],
                vec![None, Some("Unknown".into())]
            ]
        );
        assert!(
            db.query("SELECT * FROM artist_genres", &[])
                .unwrap()
                .is_empty()
        );
    }
}
