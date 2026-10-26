use super::*;
use std::sync::atomic::Ordering;

pub fn tempdir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "bcfunl-test-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&path).unwrap();
    path
}
fn args() -> Args {
    Args::try_parse_from(["bcfunl", "--count", "100"]).unwrap()
}
fn fixtures(n: usize) -> (Vec<Value>, Value) {
    let items = (0..n).map(|i| json!({"item_type":"album","item_id":i,"sale_item_type":"p","sale_item_id":i,"band_name":"Artist O'Connor","item_title":format!("Release {i}"),"purchased":"24 Oct 2025 15:30:00 GMT","release_date":"2025-10-01"})).collect();
    let urls: serde_json::Map<_, _> = (0..n)
        .map(|i| {
            (
                format!("p{i}"),
                json!(format!("https://bandcamp.com/download?fake={i}")),
            )
        })
        .collect();
    (items, Value::Object(urls))
}
#[test]
fn cookies_domain_path_expiry_and_httponly() {
    let jar = "bleep.com\tFALSE\t/\tTRUE\t0\tsecret\twrong\n#HttpOnly_.bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tok\n.bandcamp.com\tTRUE\t/api\tTRUE\t1\told\tbad\n.bandcamp.com\tTRUE\t/api\tTRUE\t0\tapi\tyes";
    assert_eq!(collection::cookies(jar, "/user", 2).unwrap(), "session=ok");
    assert_eq!(
        collection::cookies(jar, "/api/items", 2).unwrap(),
        "session=ok; api=yes"
    );
    assert_eq!(
        collection::cookies(jar, "/apiculture", 2).unwrap(),
        "session=ok"
    );
    assert!(collection::cookies("bleep.com\tFALSE\t/\tTRUE\t0\tx\ty", "/", 0).is_err());
    assert!(collection::cookies("bad", "/", 0).is_err());
}
#[test]
fn html_decoding_and_origin_validation() {
    assert_eq!(
        collection::pagedata(
            "<div id='pagedata' data-blob='{&quot;fan_data&quot;:{&quot;fan_id&quot;:42}}'></div>"
        )
        .unwrap()["fan_data"]["fan_id"],
        42
    );
    assert!(collection::pagedata("login required").is_err());
    for bad in [
        "http://bandcamp.com/x",
        "https://bandcamp.com.evil.org/x",
        "https://localhost/x",
        "https://bandcamp.com:8080/x",
        "https://user:pass@bandcamp.com/x",
    ] {
        assert!(!net::allowed(&reqwest::Url::parse(bad).unwrap(), true));
    }
    assert!(net::allowed(
        &reqwest::Url::parse("https://popplers5.bandcamp.com/download").unwrap(),
        false
    ));
    assert!(net::allowed(
        &reqwest::Url::parse("https://p4.bcbits.com/x").unwrap(),
        true
    ));
    assert!(!net::allowed(
        &reqwest::Url::parse("https://p4.bcbits.com/x").unwrap(),
        false
    ));
}
#[test]
fn named_arguments_dates_and_validation() {
    let a = Args::try_parse_from([
        "bcfunl",
        "--user",
        "stuarthunter",
        "--fromdt",
        "2025-10-24",
        "--todt",
        "2025-10-24",
        "--count",
        "100",
        "--output",
        "music",
        "--format",
        "WAV",
        "--force",
    ])
    .unwrap();
    a.validate().unwrap();
    assert_eq!(a.format, Format::Wav);
    assert!(a.force);
    assert!(a.matches(date(&json!("24 Oct 2025 15:30:00 GMT"))));
    assert!(!a.matches(date(&json!("2025-10-25"))));
    assert!(!a.matches(None));
    assert_eq!(
        date(&json!("2025-10-25T00:30:00+02:00"))
            .unwrap()
            .to_string(),
        "2025-10-24"
    );
    assert!(Args::try_parse_from(["bcfunl", "--count", "0"]).is_err());
    assert!(Args::try_parse_from(["bcfunl", "--fromdt", "2025-02-30"]).is_err());
    assert!(
        Args::try_parse_from(["bcfunl", "--fromdt", "2026-01-01", "--todt", "2025-01-01"])
            .unwrap()
            .validate()
            .is_err()
    );
}
#[test]
fn capture_skips_preorders_missing_links_duplicates_and_completed() {
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (mut items, urls) = fixtures(6);
    items[0]["is_preorder"] = json!(true);
    items[1]["is_pre_release"] = json!(1);
    items[2]["sale_item_id"] = json!(900);
    items[3]["purchased"] = json!("invalid");
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut seen, &mut selected).unwrap();
    assert_eq!(selected.len(), 3);
    capture_page(&db, &a, &items, &urls, &mut seen, &mut selected).unwrap();
    assert_eq!(selected.len(), 3);
    db.query(
        "UPDATE releases SET download_date='2025-11-01' WHERE item_key=?",
        &["album4"],
    )
    .unwrap();
    seen.clear();
    selected.clear();
    capture_page(&db, &a, &items, &urls, &mut seen, &mut selected).unwrap();
    assert_eq!(selected.len(), 2);
    let mut a = a;
    a.force = true;
    seen.clear();
    selected.clear();
    capture_page(&db, &a, &items, &urls, &mut seen, &mut selected).unwrap();
    assert_eq!(selected.len(), 3);
    assert_eq!(
        db.query("SELECT artist FROM releases WHERE item_key=?", &["album4"])
            .unwrap()[0][0]
            .as_deref(),
        Some("Artist O'Connor")
    );
}
#[test]
fn fixture_100_downloads_persist_skip_and_force() {
    let root = tempdir();
    let mut a = args();
    a.output = root.clone();
    let db_path = root.join("collection.sqlite3");
    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    let (items, urls) = fixtures(150);
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(selected.len(), 100);
    let mut calls = 0;
    process(&db, &a, &selected, |r| {
        calls += 1;
        let payload = b"fLaCfixture-audio-payload";
        let path = download::save(
            &payload[..],
            Some(payload.len() as u64),
            &root,
            &r.key,
            "flac",
            a.force,
        )?;
        Ok(download::Downloaded {
            path,
            format: "flac".into(),
            publish: r.publish,
        })
    })
    .unwrap();
    assert_eq!(calls, 100);
    assert_eq!(
        db.query(
            "SELECT count(*) FROM releases WHERE download_date IS NOT NULL",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("100")
    );
    assert_eq!(db.query("SELECT count(*) FROM releases WHERE publish_date IS NOT NULL AND purchase_date IS NOT NULL",&[]).unwrap()[0][0].as_deref(),Some("100"));
    drop(db);
    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    process(&db, &a, &selected, |_| {
        panic!("Completed records must be skipped")
    })
    .unwrap();
    a.force = true;
    let mut forced = 0;
    process(&db, &a, &selected, |r| {
        forced += 1;
        let payload = b"fLaCfixture-forced-payload";
        let path = download::save(
            &payload[..],
            Some(payload.len() as u64),
            &root,
            &r.key,
            "flac",
            true,
        )?;
        Ok(download::Downloaded {
            path,
            format: "flac".into(),
            publish: r.publish,
        })
    })
    .unwrap();
    assert_eq!(forced, 100);
    let mut dump = Vec::new();
    db.dump(&mut dump, None).unwrap();
    let dump = String::from_utf8(dump).unwrap();
    assert_eq!(dump.lines().count(), 102);
    assert!(!dump.contains("https://"));
    assert_eq!(
        fs::read_dir(&root)
            .unwrap()
            .filter(|entry| entry
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|s| s == "flac"))
            .count(),
        100
    );
    drop(db);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn failures_leave_completion_null_and_dump_escapes_cells() {
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (items, urls) = fixtures(1);
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert!(process(&db, &a, &selected, |_| bail!("Fixture download failed")).is_err());
    assert!(!db.completed(&a.user, "album0", false).unwrap());
    db.query("UPDATE releases SET artist=?", &["A\tB\nC"])
        .unwrap();
    let mut out = Vec::new();
    db.dump(&mut out, None).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("A\\tB\\nC"));
    assert!(db.query("SELECT ?", &["a\0b"]).is_err());
}

#[test]
fn parallel_argument_bounds() {
    assert_eq!(args().parallel, None);
    for value in ["2", "8"] {
        assert!(Args::try_parse_from(["bcfunl", "--parallel", value]).is_ok());
    }
    for value in ["0", "1", "9", "-2", "fast"] {
        assert!(Args::try_parse_from(["bcfunl", "--parallel", value]).is_err());
    }
}

#[test]
fn parallel_scheduler_bounds_history_failures_and_force() {
    use std::sync::{Barrier, atomic::AtomicUsize};
    for limit in [2, 8] {
        let root = tempdir();
        let db = Database::open(":memory:").unwrap();
        let mut a = args();
        a.parallel = Some(limit);
        a.output = root.clone();
        let (items, urls) = fixtures(limit as usize * 2);
        let mut selected = Vec::new();
        capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let barrier = Barrier::new(limit as usize);
        let result = process_parallel(
            &db,
            &a,
            &selected,
            |r| Ok(r.clone()),
            |r| {
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                barrier.wait();
                let result = (|| {
                    ensure!(r.key != "album0", "Fixture transfer failed");
                    let path = download::save(
                        &b"fLaCfixture-payload"[..],
                        None,
                        &root,
                        &r.key,
                        "flac",
                        false,
                    )?;
                    Ok(download::Downloaded {
                        path,
                        format: "flac".into(),
                        publish: r.publish,
                    })
                })();
                active.fetch_sub(1, Ordering::SeqCst);
                result
            },
        );
        assert!(result.is_err());
        assert_eq!(peak.load(Ordering::SeqCst), limit as usize);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(!db.completed(&a.user, "album0", false).unwrap());
        let mut prepared = 0;
        process_parallel(
            &db,
            &a,
            &selected,
            |r| {
                prepared += 1;
                Ok(r.clone())
            },
            |r| {
                let path = download::save(
                    &b"fLaCfixture-retry"[..],
                    None,
                    &root,
                    &r.key,
                    "flac",
                    false,
                )?;
                Ok(download::Downloaded {
                    path,
                    format: "flac".into(),
                    publish: r.publish,
                })
            },
        )
        .unwrap();
        assert_eq!(prepared, 1);
        a.force = true;
        prepared = 0;
        process_parallel(
            &db,
            &a,
            &selected,
            |r| {
                prepared += 1;
                Ok(r.clone())
            },
            |r| {
                let path = download::save(
                    &b"fLaCfixture-forced"[..],
                    None,
                    &root,
                    &r.key,
                    "flac",
                    true,
                )?;
                Ok(download::Downloaded {
                    path,
                    format: "flac".into(),
                    publish: r.publish,
                })
            },
        )
        .unwrap();
        assert_eq!(prepared, selected.len());
        let rows = db
            .query(
                "SELECT count(*) FROM releases WHERE download_date IS NOT NULL",
                &[],
            )
            .unwrap();
        assert_eq!(
            rows[0][0].as_deref(),
            Some((limit as usize * 2).to_string().as_str())
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn table_dump_shows_only_requested_columns_and_success_or_error() {
    let db = Database::open(":memory:").unwrap();
    for key in ["a-success", "b-error", "c-pending", "d-force-error"] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url) VALUES('fixture',?,'Artist','Title','https://bandcamp.com/download?secret')",&[key]).unwrap();
    }
    db.query("UPDATE releases SET download_date='2026-10-24' WHERE item_key IN ('a-success','d-force-error')",&[]).unwrap();
    db.query("UPDATE releases SET last_error='Transfer failed' WHERE item_key IN ('b-error','d-force-error')",&[]).unwrap();
    let mut dump = Vec::new();
    db.dump(&mut dump, None).unwrap();
    let dump = String::from_utf8(dump).unwrap();
    let rows: Vec<Vec<_>> = dump
        .lines()
        .filter(|line| line.contains(" | "))
        .map(|line| line.split(" | ").map(str::trim).collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec!["Artist", "Title", "Download date", "Attempts", "Last error"],
            vec!["Artist", "Title", "2026-10-24", "0", "success"],
            vec!["Artist", "Title", "NULL", "0", "Transfer failed"],
            vec!["Artist", "Title", "NULL", "0", "NULL"],
            vec!["Artist", "Title", "2026-10-24", "0", "Transfer failed"],
        ]
    );
}

#[test]
fn attempts_persist_for_failures_retries_force_and_skip() {
    let root = tempdir();
    let path = root.join("history.sqlite3");
    let db = Database::open(path.to_str().unwrap()).unwrap();
    let mut a = args();
    let (items, urls) = fixtures(1);
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert!(process(&db, &a, &selected, |_| bail!("Fixture failure")).is_err());
    assert_eq!(
        db.query("SELECT attempts FROM releases", &[]).unwrap()[0][0].as_deref(),
        Some("1")
    );
    assert!(
        db.query("SELECT last_attempt_date FROM releases", &[])
            .unwrap()[0][0]
            .is_some()
    );
    a.parallel = Some(2);
    process_parallel(
        &db,
        &a,
        &selected,
        |r| Ok(r.clone()),
        |r| {
            Ok(download::Downloaded {
                path: root.join("fixture.flac"),
                format: "flac".into(),
                publish: r.publish,
            })
        },
    )
    .unwrap();
    drop(db);
    let db = Database::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        db.query("SELECT attempts FROM releases", &[]).unwrap()[0][0].as_deref(),
        Some("2")
    );
    process(&db, &a, &selected, |_| {
        panic!("Must skip completed release")
    })
    .unwrap();
    process_parallel(
        &db,
        &a,
        &selected,
        |_| -> Result<()> { panic!("Must skip completed release") },
        |_| panic!("Must skip"),
    )
    .unwrap();
    assert_eq!(
        db.query("SELECT attempts FROM releases", &[]).unwrap()[0][0].as_deref(),
        Some("2")
    );
    a.force = true;
    assert!(
        process_parallel(
            &db,
            &a,
            &selected,
            |_| -> Result<()> { bail!("Fixture preparation failure") },
            |_| panic!("Preparation failed")
        )
        .is_err()
    );
    assert_eq!(
        db.query("SELECT attempts FROM releases", &[]).unwrap()[0][0].as_deref(),
        Some("3")
    );
    drop(db);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn legacy_history_gets_attempts_column_without_losing_completion() {
    let root = tempdir();
    let path = root.join("legacy.sqlite3");
    let db = Database::open(path.to_str().unwrap()).unwrap();
    db.query("DROP TABLE releases", &[]).unwrap();
    db.query("CREATE TABLE releases(user TEXT NOT NULL,item_key TEXT NOT NULL,artist TEXT NOT NULL,release_title TEXT NOT NULL,publish_date TEXT,purchase_date TEXT,download_date TEXT,download_url TEXT NOT NULL,format TEXT,file_path TEXT,last_error TEXT,PRIMARY KEY(user,item_key))",&[]).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_date,download_url) VALUES('fixture','album1','Artist','Title','2026-10-24','https://bandcamp.com/download?fixture')",&[]).unwrap();
    drop(db);
    for _ in 0..2 {
        let db = Database::open(path.to_str().unwrap()).unwrap();
        assert!(db.completed("fixture", "album1", false).unwrap());
        assert_eq!(
            db.query(
                "SELECT catalog_num,process_date,process_error,processed_path FROM releases",
                &[]
            )
            .unwrap()[0],
            vec![None, None, None, None]
        );
        assert_eq!(
            db.query("SELECT genre,country_iso,release_url FROM releases", &[])
                .unwrap()[0],
            vec![None, None, None]
        );
        assert_eq!(
            db.query("SELECT last_attempt_date FROM releases", &[])
                .unwrap()[0][0],
            None
        );
        assert_eq!(
            db.query("SELECT attempts FROM releases", &[]).unwrap()[0][0].as_deref(),
            Some("0")
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn summary_filters_attempts_since_start_inclusively_for_current_user() {
    let db = Database::open(":memory:").unwrap();
    let start = "2026-10-24T10:00:00.000000000Z";
    for (key, user, attempt, download, error) in [
        (
            "old",
            "fixture",
            "2026-10-24T09:59:59.999999999Z",
            "2026-10-24T09:59:59Z",
            "",
        ),
        ("success", "fixture", start, "2026-10-24T10:01:00Z", ""),
        (
            "failure",
            "fixture",
            "2026-10-24T10:01:00.000000000Z",
            "",
            "Failed this run",
        ),
        (
            "forced",
            "fixture",
            "2026-10-24T10:02:00.000000000Z",
            "2026-01-01T00:00:00Z",
            "Forced retry failed",
        ),
        ("other", "other-user", start, "2026-10-24T10:01:00Z", ""),
        ("unattempted", "fixture", "", "", ""),
    ] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,last_attempt_date,download_date,last_error) VALUES(?,?,'Artist',?,'https://bandcamp.com/download?fixture',NULLIF(?,''),NULLIF(?,''),NULLIF(?,''))",&[user,key,key,attempt,download,error]).unwrap();
    }
    let mut out = Vec::new();
    db.dump(&mut out, Some(("fixture", start))).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.lines().count(), 5);
    assert!(
        out.contains("success")
            && out.contains("Failed this run")
            && out.contains("Forced retry failed")
    );
    assert!(!out.contains(" old ") && !out.contains(" other ") && !out.contains("unattempted"));
}
#[test]
fn summary_option_and_all_format_names_parse() {
    assert!(!args().summary);
    for format in [
        "FLAC",
        "WAV",
        "ALAC",
        "aiff-lossless",
        "aac-hi",
        "vorbis",
        "mp3-320",
        "mp3-v0",
        "MP3",
    ] {
        let a = Args::try_parse_from(["bcfunl", "--summary", "--format", format]).unwrap();
        assert!(a.summary);
    }
}

#[test]
fn pretty_summary_aligns_unicode_and_escapes_terminal_controls() {
    use unicode_width::UnicodeWidthStr;
    let db = Database::open(":memory:").unwrap();
    for (key, artist) in [
        ("one", "µ-Ziq"),
        ("two", "音楽"),
        ("three", "Artist\u{1b}[31m"),
    ] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url) VALUES('fixture',?,?,'Title','https://bandcamp.com/download?fixture')",&[key,artist]).unwrap();
    }
    let mut out = Vec::new();
    db.dump(&mut out, None).unwrap();
    let out = String::from_utf8(out).unwrap();
    let positions: Vec<Vec<_>> = out
        .lines()
        .filter(|line| line.contains(" | "))
        .map(|line| {
            line.match_indices('|')
                .map(|(i, _)| line[..i].width())
                .collect()
        })
        .collect();
    assert!(positions.windows(2).all(|rows| rows[0] == rows[1]));
    assert!(out.contains("µ-Ziq") && out.contains("音楽"));
    assert!(!out.contains('\u{1b}'));
    assert!(out.contains("Artist\\u{1b}[31m"));
}

#[test]
fn cookies_are_scoped_to_bandcamp_subdomains_without_leaking_to_cdn() {
    let jar = "bandcamp.com\tFALSE\t/\tTRUE\t0\troot\troot-fixture\n.bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tsession-fixture\nartist.bandcamp.com\tFALSE\t/\tTRUE\t0\tartist\tartist-fixture\n";
    assert_eq!(
        collection::cookies_for(jar, "popplers5.bandcamp.com", "/download/album", 0)
            .unwrap()
            .unwrap(),
        "session=session-fixture"
    );
    assert_eq!(
        collection::cookies_for(jar, "artist.bandcamp.com", "/album/test", 0)
            .unwrap()
            .unwrap(),
        "session=session-fixture; artist=artist-fixture"
    );
    assert!(
        collection::cookies_for(jar, "p4.bcbits.com", "/download/album", 0)
            .unwrap()
            .is_none()
    );
    assert!(
        collection::cookies_for(jar, "bandcamp.com.evil.org", "/", 0)
            .unwrap()
            .is_none()
    );
}
#[test]
fn failure_history_keeps_the_underlying_error_context() {
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (items, urls) = fixtures(1);
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert!(
        process(&db, &a, &selected, |_| Err(anyhow::anyhow!(
            "Bandcamp returned HTTP 403"
        )
        .context("Resolve Bandcamp audio download")))
        .is_err()
    );
    let rows = db.query("SELECT last_error FROM releases", &[]).unwrap();
    assert_eq!(
        rows[0][0].as_deref(),
        Some("Resolve Bandcamp audio download: Bandcamp returned HTTP 403")
    );
}

#[test]
fn artist_and_title_initial_caps_preserve_entirely_uppercase_names() {
    for (raw, expected) in [
        ("jim o'rourke", "Jim O'Rourke"),
        ("steamroom 68", "Steamroom 68"),
        ("SWANS", "SWANS"),
        ("2X45", "2X45"),
        ("mary in the junkyard", "Mary In The Junkyard"),
        ("o’rourke", "O’Rourke"),
        ("µ-Ziq", "µ-Ziq"),
        ("αβγ", "αβγ"),
        ("ΑΒγ", "ΑΒγ"),
        ("écho αβγ", "écho αβγ"),
        ("αβγ steamroom", "αβγ Steamroom"),
        ("SWANS αβγ", "SWANS αβγ"),
    ] {
        assert_eq!(initcap(raw), expected);
    }
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (mut items, urls) = fixtures(1);
    items[0]["band_name"] = json!("jim o'rourke");
    items[0]["item_title"] = json!("steamroom 68");
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(selected[0].artist, "Jim O'Rourke");
    assert_eq!(selected[0].title, "Steamroom 68");
    let row = db
        .query("SELECT artist,sanitized_title FROM releases", &[])
        .unwrap();
    assert_eq!(
        row[0],
        vec![Some("Jim O'Rourke".into()), Some("Steamroom 68".into())]
    );
}

#[test]
fn capture_persists_release_url_and_refreshes_completed_rows_without_losing_it() {
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (mut items, urls) = fixtures(1);
    items[0]["item_url"] = json!("https://artist.bandcamp.com/album/first");
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(
        selected[0].release_url.as_deref(),
        Some("https://artist.bandcamp.com/album/first")
    );
    assert_eq!(
        db.query("SELECT release_url FROM releases", &[]).unwrap()[0][0].as_deref(),
        Some("https://artist.bandcamp.com/album/first")
    );
    db.query("UPDATE releases SET download_date='completed'", &[])
        .unwrap();
    items[0]["item_url"] = json!("https://artist.bandcamp.com/album/updated");
    selected.clear();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert!(selected.is_empty());
    items[0]["item_url"] = Value::Null;
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(
        db.query("SELECT release_url,download_date FROM releases", &[])
            .unwrap()[0],
        vec![
            Some("https://artist.bandcamp.com/album/updated".into()),
            Some("completed".into())
        ]
    );
}

#[test]
fn custom_release_domains_use_the_bandcamp_subdomain_and_persist_it() {
    for (kind, slug) in [("album", "feral-grace"), ("track", "soft-collapse")] {
        let item = json!({"item_url": format!("https://duststoredigital.com/{kind}/{slug}?from=collection"), "url_hints": {"subdomain": "theblackdog"}});
        assert_eq!(
            release_page_url(&item).unwrap().unwrap(),
            format!("https://theblackdog.bandcamp.com/{kind}/{slug}?from=collection")
        );
    }
    for subdomain in [
        "",
        "evil.org/path",
        "evil.bandcamp.com",
        "user@host",
        "-bad",
    ] {
        assert!(release_page_url(&json!({"item_url":"https://custom.example/album/test", "url_hints":{"subdomain":subdomain}})).is_err());
    }
    let db = Database::open(":memory:").unwrap();
    let (mut items, urls) = fixtures(1);
    items[0]["item_url"] = json!("https://duststoredigital.com/album/liber-eleven");
    items[0]["url_hints"] = json!({"subdomain":"theblackdog"});
    let mut selected = Vec::new();
    capture_page(
        &db,
        &args(),
        &items,
        &urls,
        &mut HashSet::new(),
        &mut selected,
    )
    .unwrap();
    assert_eq!(
        selected[0].release_url.as_deref(),
        Some("https://theblackdog.bandcamp.com/album/liber-eleven")
    );
    assert_eq!(
        db.query("SELECT release_url FROM releases", &[]).unwrap()[0][0],
        selected[0].release_url
    );
}

#[test]
fn release_custom_host_requires_verified_matching_collection_metadata() {
    let mut item = json!({"item_url":"https://duststoredigital.com/album/feral-grace", "url_hints":{"custom_domain":"duststoredigital.com","custom_domain_verified":1}});
    assert_eq!(
        release_custom_host(&item).as_deref(),
        Some("duststoredigital.com")
    );
    item["url_hints"]["custom_domain_verified"] = json!(0);
    assert_eq!(release_custom_host(&item), None);
    item["url_hints"]["custom_domain_verified"] = json!(true);
    item["url_hints"]["custom_domain"] = json!("other.example");
    assert_eq!(release_custom_host(&item), None);
}

#[test]
fn artist_capture_filters_before_count_and_persists_clean_identity() {
    let db = Database::open(":memory:").unwrap();
    let mut a = args();
    a.artist = Some("the xx".into());
    a.count = Some(1);
    let (mut items, urls) = fixtures(3);
    items[0]["band_name"] = json!("Different");
    items[1]["band_name"] = json!("the xx");
    items[1]["item_title"] = json!("it's time [catalo546] ");
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].artist, "The XX");
    assert_eq!(selected[0].title, "It's Time");
    let rows = db
        .query(
            "SELECT release_title,sanitized_title,catalog_num FROM releases",
            &[],
        )
        .unwrap();
    assert_eq!(
        rows[0],
        vec![
            Some("it's time [catalo546] ".into()),
            Some("It's Time".into()),
            Some("CATALO546".into())
        ]
    );
}
#[test]
fn migration_backfills_title_without_changing_download_history() {
    let root = tempdir();
    let path = root.join("history.sqlite3");
    {
        let db = Database::open(path.to_str().unwrap()).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,attempts) VALUES('u','a1','the xx','It''S Time (catalo546) ','https://bandcamp.com/download','complete',4)",&[]).unwrap();
    }
    let db = Database::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        db.query(
            "SELECT artist,sanitized_title,catalog_num,download_date,attempts FROM releases",
            &[]
        )
        .unwrap()[0],
        vec![
            Some("The XX".into()),
            Some("It's Time".into()),
            Some("CATALO546".into()),
            Some("complete".into()),
            Some("4".into())
        ]
    );
    drop(db);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn private_collection_purchases_remain_eligible_without_public_metadata() {
    let db = Database::open(":memory:").unwrap();
    let a = args();
    let (mut items, urls) = fixtures(2);
    items[0]["is_private"] = json!(true);
    items[0]["item_url"] = json!("https://daintees.bandcamp.com/album/juvenilia-2");
    items[1]["is_private"] = json!(false);
    items[1]["item_url"] = json!("https://daintees.bandcamp.com/album/howdy-honcho");
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(selected.len(), 2);
    assert!(selected[0].private);
    assert!(!selected[1].private);
    assert_eq!(
        selected[0].release_url.as_deref(),
        Some("https://daintees.bandcamp.com/album/juvenilia-2")
    );
    assert_eq!(
        db.query(
            "SELECT release_url FROM releases WHERE item_key='album0'",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("https://daintees.bandcamp.com/album/juvenilia-2")
    );
    items[0]["is_preorder"] = json!(true);
    let mut selected = Vec::new();
    capture_page(&db, &a, &items, &urls, &mut HashSet::new(), &mut selected).unwrap();
    assert_eq!(selected.len(), 1);
    assert!(!selected[0].private);
}
