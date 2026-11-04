use super::*;
use clap::Parser;

fn fixture(ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let filename_ext = if ext == "aac-hi" { "m4a" } else { ext };
    let path = dir.path().join(format!("audio.{filename_ext}"));
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=0.15",
    ]);
    if ext == "m4a" {
        cmd.args(["-c:a", "alac"]);
    }
    if ext == "aac-hi" {
        cmd.args(["-c:a", "aac"]);
    }
    command(cmd.arg(&path)).unwrap();
    (dir, path)
}
fn sample() -> Tags {
    let mut tags = Tags::new();
    for (key, text) in [
        ("ALBUM", "Proof"),
        ("ALBUMARTIST", "Romy"),
        ("ARTIST", "Romy & Peaches"),
        ("TITLE", "Did I (Peaches Remix)"),
        ("TRACKNUMBER", "1"),
        ("DISCNUMBER", "1"),
        ("ARTISTSORT", "Madley Croft, Romy | Peaches"),
        ("COMPOSER", "Original Composer"),
        ("CATALOGNUMBER", "CAT01"),
    ] {
        put(&mut tags, key, text);
    }
    tags
}
fn release(path: PathBuf) -> Release {
    Release {
        key: "album1".into(),
        artist: "Romy".into(),
        album: "Proof".into(),
        source: path,
        genre: "electronic;alternative".into(),
        country: "GB".into(),
        catalog: String::new(),
        publish: "2026-10-25".into(),
        url: "https://example.bandcamp.com/album/proof".into(),
    }
}
#[test]
fn processing_normalizes_combined_genres_with_shared_map() {
    let (_dir, path) = fixture("flac");
    let mut r = release(path.clone());
    r.genre = "Electronic;Acid House;Progressive House;acid house".into();
    let mut tracks = vec![Track {
        path,
        tags: sample(),
        duration: 0.15,
    }];
    tracks[0]
        .tags
        .insert("GENRE".into(), vec!["Synth Pop;Unmapped Example".into()]);
    prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(
        tracks[0].tags["GENRE"],
        [
            "Synth-Pop",
            "Unmapped Example",
            "Electronic",
            "Acid House",
            "House",
            "Progressive House"
        ]
    );
    r.genre.clear();
    prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(
        tracks[0].tags["GENRE"],
        [
            "Synth-Pop",
            "Unmapped Example",
            "Electronic",
            "Acid House",
            "House",
            "Progressive House"
        ]
    );
}
#[test]
fn native_tagging_preserves_multiple_credits_sort_names_and_audio() {
    for ext in ["flac", "wav", "mp3"] {
        let (_dir, path) = fixture(ext);
        let mut tags = sample();
        credits::normalize_artists(&mut tags);
        tags.insert("PERFORMER".into(), vec!["Romy".into(), "Guest".into()]);
        tags.insert(
            "GENRE".into(),
            vec!["electronic".into(), "alternative".into()],
        );
        let before = pcm_hash(&path).unwrap();
        tagging::write(&path, &tags).unwrap();
        verify_tags(&tags, &path).unwrap();
        assert_eq!(pcm_hash(&path).unwrap(), before);
        let (tags, _) = tagging::read(&path).unwrap();
        assert_eq!(tags["ARTIST"], ["Romy", "Peaches"]);
        assert_eq!(tags["REMIXER"], ["Peaches"]);
        assert_eq!(tags["ARTISTSORT"], ["Madley Croft, Romy", "Peaches"]);
        assert_eq!(tags["PERFORMER"], ["Romy", "Guest"]);
        assert_eq!(tags["COMPOSER"], ["Original Composer"]);
    }
}
#[test]
fn other_download_containers_roundtrip_metadata_without_changing_audio() {
    for ext in ["aiff", "m4a", "aac-hi", "ogg"] {
        let (_dir, path) = fixture(ext);
        let mut tags = sample();
        credits::normalize_artists(&mut tags);
        tags.insert(
            "GENRE".into(),
            vec!["electronic".into(), "alternative".into()],
        );
        let before = pcm_hash(&path).unwrap();
        tagging::write(&path, &tags).unwrap();
        verify_tags(&tags, &path).unwrap();
        assert_eq!(pcm_hash(&path).unwrap(), before, "{ext}");
    }
}
#[test]
fn archive_members_and_publication_refuse_escape_links_and_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let staging = dir.path().join("staging");
    let dest = dir.path().join("dest");
    fs::create_dir(&staging).unwrap();
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("keep"), b"original").unwrap();
    assert!(publish(&staging, &dest).is_err());
    assert_eq!(fs::read(dest.join("keep")).unwrap(), b"original");
    std::os::unix::fs::symlink(&dest, dir.path().join("alias")).unwrap();
    assert!(secure_parent(dir.path(), Path::new("alias/nested")).is_err());
}
#[test]
fn processing_zip_records_completion_and_skips_verified_server_audio() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input");
    let server = dir.path().join("server");
    fs::create_dir(&input).unwrap();
    fs::create_dir(&server).unwrap();
    let (_audio, path) = fixture("flac");
    tagging::write(&path, &sample()).unwrap();
    let archive = input.join("release.zip");
    let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
    zip.start_file(
        path.file_name().unwrap().to_str().unwrap(),
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    crate::archive::copy(File::open(&path).unwrap(), &mut zip).unwrap();
    zip.finish().unwrap();
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,genre,country_iso) VALUES('fixture','album1','Romy','Proof','https://bandcamp.com/download','complete',?,'electronic;alternative','GB')",&[archive.to_str().unwrap()]).unwrap();
    drop(db);
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
        "--summary",
    ])
    .unwrap();
    run(&args).unwrap();
    let dest = server.join("Romy/Proof [GB] [CAT01]/01-01 - Did I (Peaches Remix).flac");
    assert!(dest.exists());
    assert!(archive.exists());
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    let rows = db
        .query(
            "SELECT process_date,process_error,download_date,attempts,catalog_num FROM releases",
            &[],
        )
        .unwrap();
    assert!(rows[0][0].is_some());
    assert_eq!(rows[0][1], None);
    assert_eq!(rows[0][2].as_deref(), Some("complete"));
    assert_eq!(rows[0][3].as_deref(), Some("0"));
    assert_eq!(rows[0][4].as_deref(), Some("CAT01"));
    let original = fs::read(&dest).unwrap();
    run(&args).unwrap();
    assert_eq!(fs::read(&dest).unwrap(), original);
    // Simulate a prior tool's different suffix and filename with intact audio/tags.
    let old = server.join("Romy/Proof [US] [OLD01]");
    fs::rename(dest.parent().unwrap(), &old).unwrap();
    fs::rename(
        old.join(dest.file_name().unwrap()),
        old.join("older-name.flac"),
    )
    .unwrap();
    db.query(
        "UPDATE releases SET process_date=NULL,processed_path=NULL",
        &[],
    )
    .unwrap();
    run(&args).unwrap();
    assert!(!dest.exists());
    assert_eq!(
        db.query("SELECT processed_path FROM releases", &[])
            .unwrap()[0][0]
            .as_deref(),
        Some(old.to_str().unwrap())
    );
    let mut forced = args;
    forced.force = true;
    run(&forced).unwrap();
    assert!(old.join("older-name.flac").exists());
}
#[test]
fn compilation_plan_preserves_per_track_performers_and_missing_catalog() {
    let (_dir, path) = fixture("flac");
    let mut first = Track {
        path: path.clone(),
        tags: sample(),
        duration: 0.15,
    };
    first.tags.remove("ALBUMARTIST");
    first.tags.remove("CATALOGNUMBER");
    put(&mut first.tags, "ARTIST", "bvdub | Brock Van Wey");
    let mut second = first.clone();
    put(&mut second.tags, "ARTIST", "Different Performer");
    put(&mut second.tags, "TRACKNUMBER", 2);
    let mut tracks = vec![first, second];
    let plan = prepare_tags(
        &release(path),
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.artist, "Various Artists");
    assert!(plan.catalog.is_empty());
    assert_eq!(
        tracks[0].tags["TRACKARTIST"],
        ["bvdub", "Brock Van Wey", "Peaches"]
    );
    assert_eq!(value(&tracks[1].tags, "TRACKARTIST"), "Different Performer");
    assert_eq!(value(&tracks[0].tags, "ALBUMARTIST"), "Various Artists");
    assert_eq!(plan.directory, PathBuf::from("Various Artists/Proof [GB]"));
}
#[test]
fn processing_arguments_require_server_and_do_not_load_cookies() {
    assert!(Args::try_parse_from(["bcfunl", "--process"]).is_err());
    assert!(Args::try_parse_from(["bcfunl", "--serverdir", "server"]).is_err());
    assert!(Args::try_parse_from(["bcfunl", "--template", template::DEFAULT]).is_err());
    let (_dir, path) = fixture("flac");
    let mut r = release(path);
    r.country.clear();
    r.genre.clear();
    let mut tracks = vec![Track {
        path: r.source.clone(),
        tags: sample(),
        duration: 0.15,
    }];
    put(&mut tracks[0].tags, "CATALOGNUMBER", "none");
    let plan = prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.directory, PathBuf::from("Romy/Proof"));
    assert!(!tracks[0].tags.contains_key("CATALOGNUMBER"));
}

#[test]
fn missing_metadata_does_not_mark_processing_complete_or_change_downloads() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let server = root.path().join("server");
    fs::create_dir(&input).unwrap();
    let (_fixture, audio) = fixture("flac");
    let source = input.join("01.flac");
    fs::copy(audio, &source).unwrap();
    let original = fs::read(&source).unwrap();
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,attempts) VALUES('fixture','album1','Artist','Album','https://bandcamp.com/download','complete',?,7)", &[source.to_str().unwrap()]).unwrap();
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
        "--cookies",
        "/nonexistent/no-network",
    ])
    .unwrap();
    assert!(run(&args).is_err());
    let row = &db
        .query(
            "SELECT process_date,process_error,download_date,last_error,attempts FROM releases",
            &[],
        )
        .unwrap()[0];
    assert_eq!(row[0], None);
    assert!(row[1].is_some());
    assert_eq!(row[2].as_deref(), Some("complete"));
    assert_eq!(row[3], None);
    assert_eq!(row[4].as_deref(), Some("7"));
    assert_eq!(fs::read(source).unwrap(), original);
    assert!(!server.join("Artist").exists());
    assert!(!fs::read_dir(server).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".bcfunl-process-")
    }));
}

#[test]
fn copied_credit_rules_do_not_split_band_names_or_invent_remixers() {
    for (text, expected) in [
        ("Artist feat. Guest", vec!["Artist", "Guest"]),
        ("Earth, Wind & Fire", vec!["Earth, Wind & Fire"]),
        ("Man With No Name", vec!["Man With No Name"]),
        ("AC/DC", vec!["AC/DC"]),
        (
            "Kevin Richard Martin / Dis Fig",
            vec!["Kevin Richard Martin", "Dis Fig"],
        ),
    ] {
        assert_eq!(credits::artist_names(text), expected);
    }
    for (title, expected) in [
        ("Song (Forest Drive West Remix)", vec!["Forest Drive West"]),
        ("Song [remixed by DJ Phil]", vec!["DJ Phil"]),
        ("Song (Peaches Extended Remix)", vec!["Peaches"]),
        ("Song (Club Remix)", vec![]),
        ("An Unqualified Remix", vec![]),
    ] {
        let mut tags = Tags::new();
        put(&mut tags, "ARTIST", "Artist");
        put(&mut tags, "TITLE", title);
        credits::normalize_artists(&mut tags);
        assert_eq!(tags.get("REMIXER").cloned().unwrap_or_default(), expected);
        let once = tags.clone();
        credits::normalize_artists(&mut tags);
        assert_eq!(tags, once);
    }
}

#[test]
fn matching_names_with_different_audio_are_not_skipped_or_overwritten() {
    let (source_dir, path) = fixture("flac");
    tagging::write(&path, &sample()).unwrap();
    let server = tempfile::tempdir().unwrap();
    let dest = server
        .path()
        .join("Romy/Proof [GB] [CAT01]/01-01 - Did I (Peaches Remix).flac");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    command(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=880:duration=0.15",
            ])
            .arg(&dest),
    )
    .unwrap();
    tagging::write(&dest, &sample()).unwrap();
    let original = fs::read(&dest).unwrap();
    let result = process_one(
        &release(path),
        source_dir.path(),
        server.path(),
        &template::Template::new(template::DEFAULT).unwrap(),
        None,
        None,
        false,
    );
    assert!(result.is_err());
    assert_eq!(fs::read(&dest).unwrap(), original);
}

#[test]
fn embedded_artwork_survives_tagging_and_supplies_folder_image() {
    let (input, path) = fixture("flac");
    tagging::write(&path, &sample()).unwrap();
    let picture = input.path().join("cover.jpg");
    command(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=16x16",
                "-frames:v",
                "1",
                "-threads",
                "1",
            ])
            .arg(&picture),
    )
    .unwrap();
    command(
        Command::new("metaflac")
            .arg(format!("--import-picture-from={}", picture.display()))
            .arg(&path),
    )
    .unwrap();
    let server = tempfile::tempdir().unwrap();
    let (destination, _, _) = process_one(
        &release(path),
        input.path(),
        server.path(),
        &template::Template::new(template::DEFAULT).unwrap(),
        None,
        None,
        false,
    )
    .unwrap();
    assert!(
        fs::read(destination.join("folder.jpg"))
            .unwrap()
            .starts_with(b"\xff\xd8")
    );
    assert!(
        tagging::picture(&destination.join("01-01 - Did I (Peaches Remix).flac"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_new_download_invalidates_old_processing_completion() {
    let db = Database::open(":memory:").unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,process_date,processed_path,process_error) VALUES('fixture','album1','Artist','Album','https://bandcamp.com/download','old','old','old-path','old-error')",&[]).unwrap();
    db.query("UPDATE releases SET genre_fallback='old provenance'", &[])
        .unwrap();
    let args = Args::try_parse_from(["bcfunl", "--user", "fixture"]).unwrap();
    let source = crate::Release {
        key: "album1".into(),
        artist: "Artist".into(),
        title: "Album".into(),
        url: "https://bandcamp.com/download".into(),
        publish: None,
        release_url: None,
        release_custom_host: None,
        private: false,
    };
    crate::record_result(
        &db,
        &args,
        &source,
        Ok(crate::download::Downloaded {
            path: PathBuf::from("new-file"),
            format: "flac".into(),
            publish: None,
        }),
    )
    .unwrap();
    assert_eq!(
        db.query(
            "SELECT process_date,processed_path,process_error,genre_fallback FROM releases",
            &[]
        )
        .unwrap()[0],
        vec![None, None, None, None]
    );
    assert!(db.completed("fixture", "album1", false).unwrap());
}

#[test]
fn label_compilation_uses_various_artists_and_retains_the_label() {
    let (_dir, path) = fixture("flac");
    let mut tracks = Vec::new();
    for (index, name) in [(1, "Artist A"), (2, "Artist B")] {
        let mut tags = sample();
        put(&mut tags, "ALBUMARTIST", "Optimo Music");
        put(&mut tags, "ALBUMARTISTSORT", "Optimo Music");
        put(&mut tags, "ARTIST", name);
        put(&mut tags, "TITLE", format!("{name} - Track {index}"));
        put(&mut tags, "TRACKNUMBER", index);
        tracks.push(Track {
            path: path.clone(),
            tags,
            duration: 0.15,
        });
    }
    let plan = prepare_tags(
        &release(path),
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.artist, "Various Artists");
    assert_eq!(value(&tracks[0].tags, "TITLE"), "Track 1");
    assert_eq!(value(&tracks[1].tags, "TRACKARTIST"), "Artist B");
    assert_eq!(value(&tracks[0].tags, "LABEL"), "Optimo Music");
    assert_eq!(value(&tracks[0].tags, "ALBUMARTISTSORT"), "Various Artists");
}

#[test]
fn audio_identity_includes_sample_rate_and_channels() {
    let dir = tempfile::tempdir().unwrap();
    let mut hashes = Vec::new();
    for (name, layout, duration) in [("mono", "mono", "0.2"), ("stereo", "stereo", "0.1")] {
        let path = dir.path().join(format!("{name}.flac"));
        command(
            Command::new("ffmpeg")
                .args(["-v", "error", "-f", "lavfi", "-i"])
                .arg(format!("anullsrc=r=44100:cl={layout}"))
                .args(["-t", duration])
                .arg(&path),
        )
        .unwrap();
        hashes.push(pcm_hash(&path).unwrap());
    }
    assert_ne!(hashes[0], hashes[1]);
}

#[test]
fn a_known_single_performer_fills_missing_album_artist_before_storefront() {
    let (_dir, path) = fixture("flac");
    let mut tags = sample();
    tags.remove("ALBUMARTIST");
    put(&mut tags, "ARTIST", "Actual Performer");
    let mut r = release(path.clone());
    r.artist = "Storefront Label".into();
    let mut tracks = vec![Track {
        path,
        tags,
        duration: 0.15,
    }];
    let plan = prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.artist, "Actual Performer");
}

#[test]
fn david_holmes_album_credits_and_remix_case_roundtrip() {
    for (raw, volume, title, remixer) in [
        (
            "David Holmes  David Holmes Ft. Raven Violet - Blind On A Galloping Horse (Remixes Vol. 1)",
            1,
            "Agitprop 13 (GLOK Remix)",
            "GLOK",
        ),
        (
            "David Holmes Ft. Raven Violet - Blind On A Galloping Horse (Remixes Vol. 2)",
            2,
            "Yeah x 3 - Jordan Nocturne Remix",
            "Jordan Nocturne",
        ),
    ] {
        let (_dir, path) = fixture("flac");
        let mut tags = Tags::new();
        for (key, text) in [
            ("ARTIST", "David Holmes"),
            ("ALBUMARTIST", "David Holmes"),
            ("ALBUM", raw),
            ("TITLE", title),
            ("TRACKNUMBER", "8"),
        ] {
            put(&mut tags, key, text);
        }
        tagging::write(&path, &tags).unwrap();
        let (raw_tags, duration) = tagging::read(&path).unwrap();
        assert_eq!(raw_tags["ARTIST"], ["David Holmes"]);
        assert!(!raw_tags.contains_key("REMIXER"));
        let mut source = release(path.clone());
        source.artist = "David Holmes".into();
        source.album = raw.into();
        let mut tracks = vec![Track {
            path: path.clone(),
            tags: raw_tags,
            duration,
        }];
        let plan = prepare_tags(
            &source,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        assert_eq!(
            plan.directory,
            PathBuf::from(format!(
                "David Holmes/Blind On A Galloping Horse (Remixes Vol. {volume}) [GB]"
            ))
        );
        let tags = &tracks[0].tags;
        for key in ["ARTIST", "ALBUMARTIST", "ARTISTSORT", "ALBUMARTISTSORT"] {
            assert_eq!(tags[key], ["David Holmes"], "{key}");
        }
        assert_eq!(
            tags["TRACKARTIST"],
            ["David Holmes", "Raven Violet", remixer]
        );
        assert_eq!(tags["PERFORMER"], tags["TRACKARTIST"]);
        assert_eq!(tags["REMIXER"], [remixer]);
        tagging::write(&path, tags).unwrap();
        verify_tags(tags, &path).unwrap();
    }
}

#[test]
fn album_cleanup_is_conservative_and_captured_guests_fill_matching_plain_album() {
    for (raw, expected) in [
        ("David Holmes", "David Holmes"),
        ("David Holmes Is Great", "David Holmes Is Great"),
        ("David Holmes - Proof", "Proof"),
        ("David Holmes (featuring Raven Violet) - Proof", "Proof"),
    ] {
        assert_eq!(credits::album_credit(raw, "David Holmes").0, expected);
    }
    let (_dir, path) = fixture("flac");
    let mut tags = sample();
    put(&mut tags, "ARTIST", "David Holmes");
    put(&mut tags, "ALBUMARTIST", "David Holmes");
    put(&mut tags, "ALBUM", "Proof");
    put(&mut tags, "TITLE", "Track");
    let mut source = release(path.clone());
    source.artist = "David Holmes".into();
    source.album = "David Holmes ft. Raven Violet - Proof".into();
    let mut tracks = vec![Track {
        path,
        tags,
        duration: 0.15,
    }];
    prepare_tags(
        &source,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(
        tracks[0].tags["TRACKARTIST"],
        ["David Holmes", "Raven Violet"]
    );
    assert_eq!(tracks[0].tags["PERFORMER"], tracks[0].tags["TRACKARTIST"]);
}

#[test]
fn remix_case_changes_do_not_add_duplicate_credits() {
    let mut tags = Tags::new();
    put(&mut tags, "ARTIST", "David Holmes");
    put(&mut tags, "TITLE", "Agitprop 13 (GLOK Remix)");
    credits::normalize_artists(&mut tags);
    put(&mut tags, "TITLE", "Agitprop 13 (Glok Remix)");
    credits::normalize_artists(&mut tags);
    assert_eq!(tags["REMIXER"], ["GLOK"]);
    assert_eq!(tags["ARTIST"], ["David Holmes", "GLOK"]);
}

#[test]
fn offline_artist_filter_leaves_other_releases_unattempted() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    fs::create_dir(&input).unwrap();
    let server = root.path().join("server");
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('fixture','a1','Other Artist','Proof','https://bandcamp.com/download','complete','missing.zip')",&[]).unwrap();
    drop(db);
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--user",
        "fixture",
        "--artist",
        "The XX",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
    ])
    .unwrap();
    run(&args).unwrap();
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    assert_eq!(
        db.query(
            "SELECT process_date,process_error,attempts FROM releases",
            &[]
        )
        .unwrap()[0],
        vec![None, None, Some("0".into())]
    );
}

#[test]
fn processing_extracts_catalog_from_embedded_album_and_uses_clean_name() {
    let (_dir, path) = fixture("flac");
    let mut r = release(path.clone());
    r.catalog.clear();
    r.artist = "David Holmes".into();
    let mut tags = sample();
    tags.remove("CATALOGNUMBER");
    put(&mut tags, "ALBUMARTIST", "David Holmes");
    put(&mut tags, "ARTIST", "David Holmes");
    put(
        &mut tags,
        "ALBUM",
        "David Holmes The Remixes - It's Time [catalo546]",
    );
    let mut tracks = vec![Track {
        path,
        tags,
        duration: 0.15,
    }];
    let plan = prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.catalog, "CATALO546");
    assert_eq!(
        plan.directory,
        PathBuf::from("David Holmes/It's Time [GB] [CATALO546]")
    );
}

#[test]
fn processing_dates_and_count_limit_attempts_after_sql_artist_selection() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    fs::create_dir(&input).unwrap();
    let server = root.path().join("server");
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    for (id, artist, date) in [
        ("new", "The XX", "2026-10-25"),
        ("old", "The XX", "2026-10-24"),
        ("outside", "The XX", "2026-10-26"),
        ("unknown", "The XX", ""),
        ("other", "Other", "2026-10-25"),
    ] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,purchase_date) VALUES('fixture',?,?,'Proof','https://bandcamp.com/download','complete','missing.zip',NULLIF(?,''))",&[id,artist,date]).unwrap();
    }
    drop(db);
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--user",
        "fixture",
        "--artist",
        "the xx",
        "--fromdt",
        "2026-10-24",
        "--todt",
        "2026-10-25",
        "--count",
        "1",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
    ])
    .unwrap();
    assert!(run(&args).is_err());
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    assert_eq!(
        db.query(
            "SELECT item_key FROM releases WHERE process_error IS NOT NULL",
            &[]
        )
        .unwrap(),
        vec![vec![Some("new".into())]]
    );
    assert!(
        db.query("SELECT item_key FROM releases WHERE attempts<>0", &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn staged_query_supports_every_artist_and_date_filter_combination() {
    let db = Database::open(":memory:").unwrap();
    for (id, artist, date) in [
        ("early", "The XX", "2026-10-23"),
        ("from", "The XX", "2026-10-24"),
        ("to", "The XX", "2026-10-25"),
        ("late", "The XX", "2026-10-26"),
        ("missing", "The XX", ""),
        ("other", "Other", "2026-10-25"),
        ("literal", "100%_Artist", "2026-10-25"),
    ] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,purchase_date) VALUES('fixture',?,?,'Proof','https://bandcamp.com/download','complete',NULLIF(?,''))",&[id,artist,date]).unwrap();
    }
    for artist in [false, true] {
        for (from, to, expected) in [
            (None, None, if artist { 5 } else { 7 }),
            (Some("2026-10-24"), None, if artist { 3 } else { 5 }),
            (None, Some("2026-10-25"), if artist { 3 } else { 5 }),
            (
                Some("2026-10-24"),
                Some("2026-10-25"),
                if artist { 2 } else { 4 },
            ),
        ] {
            let mut args = Args::try_parse_from(["bcfunl", "--user", "fixture"]).unwrap();
            args.artist = artist.then(|| "the xx".into());
            args.fromdt = from.map(|s| s.parse().unwrap());
            args.todt = to.map(|s| s.parse().unwrap());
            let rows = processing_rows(&db, &args).unwrap();
            assert_eq!(
                rows.len(),
                expected,
                "artist={artist},from={from:?},to={to:?}"
            );
            assert_eq!(
                rows.iter().any(|r| r[0].as_deref() == Some("missing")),
                from.is_none() && to.is_none()
            );
        }
    }
    let mut args =
        Args::try_parse_from(["bcfunl", "--user", "fixture", "--artist", "100%_"]).unwrap();
    assert_eq!(processing_rows(&db, &args).unwrap().len(), 1);
    args.artist = Some("t_e xx".into());
    assert_eq!(processing_rows(&db, &args).unwrap().len(), 5);
    args.artist = Some("the%xx".into());
    assert_eq!(processing_rows(&db, &args).unwrap().len(), 5);
    args.artist = Some("' OR 1=1 --".into());
    assert!(processing_rows(&db, &args).unwrap().is_empty());
}

#[test]
fn single_audio_defaults_missing_numbers_but_multiple_tracks_require_numbering() {
    let (_dir, path) = fixture("flac");
    let r = release(path.clone());
    let mut tags = sample();
    tags.remove("TRACKNUMBER");
    tags.remove("DISCNUMBER");
    let track = Track {
        path,
        tags,
        duration: 0.15,
    };
    let template = template::Template::new(template::DEFAULT).unwrap();
    let mut multiple = vec![track.clone(), track.clone()];
    assert!(
        prepare_tags(&r, &mut multiple, &template)
            .err()
            .unwrap()
            .to_string()
            .contains("Missing track number")
    );
    let mut single = vec![track.clone()];
    let plan = prepare_tags(&r, &mut single, &template).unwrap();
    assert_eq!(value(&single[0].tags, "TRACKNUMBER"), "1");
    assert_eq!(value(&single[0].tags, "DISCNUMBER"), "1");
    for field in ["TRACKTOTAL", "TOTALTRACKS", "DISCTOTAL", "TOTALDISCS"] {
        assert_eq!(value(&single[0].tags, field), "1", "{field}");
    }
    assert!(
        plan.files[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("01-01 - ")
    );
    let mut numbered = vec![track.clone()];
    put(&mut numbered[0].tags, "TRACKNUMBER", 7);
    put(&mut numbered[0].tags, "DISCNUMBER", 2);
    prepare_tags(&r, &mut numbered, &template).unwrap();
    assert_eq!(value(&numbered[0].tags, "TRACKNUMBER"), "7");
    assert_eq!(value(&numbered[0].tags, "DISCNUMBER"), "2");
    let mut invalid = vec![track];
    put(&mut invalid[0].tags, "TRACKNUMBER", "invalid");
    assert!(prepare_tags(&r, &mut invalid, &template).is_err());
}

#[test]
fn mapped_collaborations_keep_separate_credits_and_canonical_case() {
    for source in ["Bvdub . Brock Van Wey", "bvdub / Brock Van Wey"] {
        let (_dir, path) = fixture("flac");
        let mut r = release(path.clone());
        r.artist = source.into();
        r.album = "Farewell".into();
        let mut tags = sample();
        tags.remove("ARTISTSORT");
        tags.remove("REMIXER");
        put(&mut tags, "ARTIST", source);
        put(&mut tags, "ALBUMARTIST", source);
        put(&mut tags, "ALBUM", "Farewell");
        put(&mut tags, "TITLE", "The bvdub Remix");
        let mut tracks = vec![Track {
            path: path.clone(),
            tags,
            duration: 0.15,
        }];
        let plan = prepare_tags(
            &r,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        assert_eq!(plan.artist, "bvdub");
        for field in ["ARTIST", "ARTISTSORT", "TRACKARTIST", "PERFORMER"] {
            assert_eq!(tracks[0].tags[field], ["bvdub", "Brock Van Wey"], "{field}");
        }
        assert_eq!(tracks[0].tags["ALBUMARTIST"], ["bvdub"]);
        assert_eq!(tracks[0].tags["TITLE"], ["The bvdub Remix"]);
        tagging::write(&path, &tracks[0].tags).unwrap();
        verify_tags(&tracks[0].tags, &path).unwrap();
    }
    assert_eq!(
        credits::artist_names("BP Fallon & David Holmes"),
        ["BP Fallon", "David Holmes"]
    );
    assert_eq!(
        credits::artist_names("Earth, Wind & Fire"),
        ["Earth, Wind & Fire"]
    );
    assert_eq!(credits::artist_names("Some. Band"), ["Some. Band"]);
}

#[test]
fn mapped_title_phrase_moves_only_to_affected_track_credits() {
    let (_dir, path) = fixture("flac");
    let mut r = release(path.clone());
    r.artist = "BP Fallon & David Holmes".into();
    let mut tags = sample();
    tags.remove("REMIXER");
    tags.remove("ARTISTSORT");
    put(&mut tags, "ARTIST", "BP Fallon & David Holmes");
    put(&mut tags, "ALBUMARTIST", "BP Fallon & David Holmes");
    put(
        &mut tags,
        "TITLE",
        "Song - Henry Mccullough Andrew Weatherall",
    );
    let first = Track {
        path: path.clone(),
        tags,
        duration: 0.15,
    };
    let mut second = first.clone();
    put(&mut second.tags, "TRACKNUMBER", 2);
    put(&mut second.tags, "TITLE", "Another Song");
    let mut tracks = vec![first, second];
    prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(tracks[0].tags["TITLE"], ["Song"]);
    for field in ["TRACKARTIST", "PERFORMER"] {
        assert_eq!(
            tracks[0].tags[field],
            [
                "BP Fallon",
                "David Holmes",
                "Henry McCullough",
                "Andrew Weatherall"
            ]
        );
        assert_eq!(tracks[1].tags[field], ["BP Fallon", "David Holmes"]);
    }
    assert_eq!(tracks[1].tags["TITLE"], ["Another Song"]);
    assert_eq!(tracks[0].tags["ALBUMARTIST"], ["BP Fallon"]);
    tagging::write(&path, &tracks[0].tags).unwrap();
    verify_tags(&tracks[0].tags, &path).unwrap();
    assert_eq!(
        crate::sanitizing::title_credits("Henry Mccullough Andrew Weatherall (Remixes)"),
        (
            "(Remixes)".into(),
            vec!["Henry McCullough".into(), "Andrew Weatherall".into()]
        )
    );
    assert!(
        crate::sanitizing::title_credits("Henry Mccullough Andrew Weatherallian")
            .1
            .is_empty()
    );
}

#[test]
fn self_named_and_ampersand_expansions_roundtrip_without_changing_album_artist() {
    for source in [
        "Martin Stephenson And The Daintees",
        "Martin Stephenson & The Daintees",
    ] {
        let (_dir, path) = fixture("flac");
        let mut r = release(path.clone());
        r.artist = source.into();
        let mut tags = sample();
        tags.remove("ARTISTSORT");
        tags.remove("REMIXER");
        put(&mut tags, "ARTIST", source);
        put(&mut tags, "ALBUMARTIST", source);
        put(&mut tags, "TITLE", "A Song");
        let mut tracks = vec![Track {
            path: path.clone(),
            tags,
            duration: 0.15,
        }];
        let plan = prepare_tags(
            &r,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        assert_eq!(plan.artist, "Martin Stephenson And The Daintees");
        for field in ["ARTIST", "TRACKARTIST", "PERFORMER"] {
            assert_eq!(
                tracks[0].tags[field],
                [
                    "Martin Stephenson And The Daintees",
                    "Martin Stephenson",
                    "The Daintees"
                ]
            );
        }
        assert_eq!(
            tracks[0].tags["ALBUMARTIST"],
            ["Martin Stephenson And The Daintees"]
        );
        tagging::write(&path, &tracks[0].tags).unwrap();
        verify_tags(&tracks[0].tags, &path).unwrap();
    }
}

#[test]
fn processing_missing_genres_publishes_verified_fallback_and_audit_without_promoting_release_genre()
{
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let server = root.path().join("server");
    fs::create_dir(&input).unwrap();
    fs::create_dir(input.join("resources")).unwrap();
    genres::tests::fixture_library(&input.join("resources/library.sqlite3"));
    let (_dir, audio) = fixture("flac");
    let mut tags = sample();
    tags.remove("GENRE");
    tagging::write(&audio, &tags).unwrap();
    let source = input.join("single.flac");
    fs::copy(&audio, &source).unwrap();
    let dbpath = input.join("collection.sqlite3");
    let db = Database::open(dbpath.to_str().unwrap()).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('fixture','album1','Romy','Proof','https://bandcamp.com/download','complete',?)",&[source.to_str().unwrap()]).unwrap();
    drop(db);
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--parallel",
        "2",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
    ])
    .unwrap();
    run(&args).unwrap();
    let db = Database::open(dbpath.to_str().unwrap()).unwrap();
    let rows = db
        .query(
            "SELECT genre,genre_fallback,processed_path FROM releases",
            &[],
        )
        .unwrap();
    assert_eq!(rows[0][0], None);
    assert!(rows[0][1].as_ref().unwrap().contains("artist_id"));
    let dir = PathBuf::from(rows[0][2].as_ref().unwrap());
    let paths = collect(&dir).unwrap();
    let published = tracks(&paths).unwrap();
    assert_eq!(
        published[0].tags["GENRE"],
        ["Electronic", "Ambient", "Techno", "IDM", "Drone"]
    );
    assert!(published[0].tags.contains_key("BCFUNL_GENRE_SOURCE"));
    assert_eq!(
        pcm_hash(&source).unwrap(),
        pcm_hash(&published[0].path).unwrap()
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join(".bcfunl-release.json")).unwrap()).unwrap();
    assert_eq!(manifest["genre_fallback"]["artist_id"], "1");
    drop(db);
    let mut retry = args;
    retry.library = Some(input.join("missing.sqlite3"));
    assert!(run(&retry).is_err());
}

#[test]
fn parallel_processing_preserves_count_history_skips_and_failure_isolation() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let server = root.path().join("server");
    fs::create_dir(&input).unwrap();
    let (_audio, source) = fixture("flac");
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    for index in 1..=3 {
        let path = input.join(format!("release{index}.flac"));
        fs::copy(&source, &path).unwrap();
        let mut tags = sample();
        put(&mut tags, "ALBUM", format!("Proof {index}"));
        tagging::write(&path, &tags).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('fixture',?,'Romy',?,'unused','complete',?)", &[&format!("album{index}"), &format!("Proof {index}"), path.to_str().unwrap()]).unwrap();
    }
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('fixture','album4','Romy','Missing','unused','complete','missing.flac')", &[]).unwrap();
    let mut args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--parallel",
        "2",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
        "--count",
        "2",
    ])
    .unwrap();
    run(&args).unwrap();
    assert_eq!(
        db.query(
            "SELECT COUNT(*) FROM releases WHERE process_date IS NOT NULL",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        db.query(
            "SELECT COUNT(*) FROM releases WHERE process_error IS NOT NULL",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("0")
    );
    args.count = None;
    assert!(run(&args).is_err());
    assert_eq!(
        db.query(
            "SELECT COUNT(*) FROM releases WHERE process_date IS NOT NULL",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("3")
    );
    assert_eq!(
        db.query(
            "SELECT item_key FROM releases WHERE process_error IS NOT NULL",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("album4")
    );
    assert_eq!(
        db.query(
            "SELECT COUNT(*) FROM releases WHERE download_date='complete' AND attempts=0",
            &[]
        )
        .unwrap()[0][0]
            .as_deref(),
        Some("4")
    );
    for index in 1..=3 {
        assert!(
            server
                .join(format!(
                    "Romy/Proof {index} [CAT01]/01-01 - Did I (Peaches Remix).flac"
                ))
                .is_file()
        );
        assert!(input.join(format!("release{index}.flac")).is_file());
    }
}

#[test]
fn process_and_clean_removes_only_successful_payload_and_keeps_history() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let artist = input.join("fixture/Romy");
    let server = root.path().join("server");
    fs::create_dir_all(&artist).unwrap();
    let (_audio, source) = fixture("flac");
    let payload = artist.join("proof.flac");
    fs::copy(source, &payload).unwrap();
    tagging::write(&payload, &sample()).unwrap();
    let failed = artist.join("broken.zip");
    fs::write(&failed, b"not an archive").unwrap();
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    for (key, path) in [("album1", &payload), ("album2", &failed)] {
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('fixture',?,'Romy','Proof','unused','complete',?)", &[key,path.to_str().unwrap()]).unwrap();
    }
    let mut args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--clean",
        "--parallel",
        "2",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
    ])
    .unwrap();
    assert!(run(&args).is_err());
    assert!(!payload.exists());
    assert!(failed.exists());
    let published = server.join("Romy/Proof [CAT01]/01-01 - Did I (Peaches Remix).flac");
    assert!(published.is_file());
    let rows = db.query("SELECT process_date,process_error,download_date,file_path FROM releases WHERE item_key='album1'", &[]).unwrap();
    assert!(rows[0][0].is_some());
    assert!(rows[0][1].is_none());
    assert_eq!(rows[0][2].as_deref(), Some("complete"));
    assert_eq!(rows[0][3].as_deref(), payload.to_str());
    // Standalone cleanup is idempotent and does not retry failed processing.
    super::clean::run(&args).unwrap();
    args.count = Some(1);
    // The already-published release still skips despite its removed source.
    assert!(run(&args).is_err());
    assert!(published.is_file());
}

#[test]
fn processing_leading_catalogs_preserve_album_subtitles_and_existing_catalogs() {
    let r = release(PathBuf::from("unused.flac"));
    for (album, expected_album, catalog) in [
        ("[Af015] D Funk EP", "D Funk EP", "AF015"),
        (
            "Romy - Hnrbox003: Title - Subtitle",
            "Title - Subtitle",
            "HNRBOX003",
        ),
    ] {
        for existing in [None, Some("CONFIRMED123")] {
            let mut tags = sample();
            tags.remove("CATALOGNUMBER");
            if let Some(existing) = existing {
                put(&mut tags, "CATALOGNUMBER", existing);
            }
            put(&mut tags, "ALBUM", album);
            let mut tracks = vec![Track {
                path: r.source.clone(),
                tags,
                duration: 0.15,
            }];
            let plan = prepare_tags(
                &r,
                &mut tracks,
                &template::Template::new(template::DEFAULT).unwrap(),
            )
            .unwrap();
            let expected_catalog = existing.unwrap_or(catalog);
            assert_eq!(plan.catalog, expected_catalog);
            let decorated = format!("{expected_album} [GB] [{expected_catalog}]");
            assert_eq!(plan.album, expected_album);
            assert_eq!(value(&tracks[0].tags, "ALBUM"), decorated);
            assert_eq!(value(&tracks[0].tags, "CATALOGNUMBER"), expected_catalog);
            assert_eq!(
                plan.directory,
                PathBuf::from(format!("Romy/{expected_album} [GB] [{expected_catalog}]"))
            );
        }
    }
}

#[test]
fn stale_releases_lose_explicit_prerelease_markers() {
    for (count, date, strip) in [
        (1, "2023-01-01".to_owned(), true),
        (1, chrono::Utc::now().date_naive().to_string(), false),
        (1, String::new(), false),
        (2, "2023-01-01".to_owned(), true),
        (
            2,
            (chrono::Utc::now().date_naive() - chrono::Duration::days(30)).to_string(),
            false,
        ),
        (
            2,
            (chrono::Utc::now().date_naive() - chrono::Duration::days(31)).to_string(),
            true,
        ),
    ] {
        let mut r = release(PathBuf::from("unused.flac"));
        r.publish = date;
        let mut tracks = Vec::new();
        for n in 1..=count {
            let mut tags = sample();
            put(&mut tags, "ALBUM", "Opposites (PRE-RELEASE)");
            put(&mut tags, "TITLE", "Opposites [Pre-Release]");
            put(&mut tags, "TRACKNUMBER", n.to_string());
            tracks.push(Track {
                path: r.source.clone(),
                tags,
                duration: 0.15,
            });
        }
        let plan = prepare_tags(
            &r,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        assert_eq!(plan.album.contains("Pre-Release"), !strip);
        assert_eq!(
            value(&tracks[0].tags, "TITLE").contains("Pre-Release"),
            !strip
        );
    }
    assert_eq!(
        crate::sanitizing::remove_prerelease("Pre-Release Versions (Deluxe)"),
        "Pre-Release Versions (Deluxe)"
    );
}

#[test]
fn ombra_catalog_only_album_keeps_title_and_formats_country_and_catalog() {
    let mut r = release(PathBuf::from("unused.flac"));
    r.album = "Ombra Intl 006".into();
    r.country = "DE".into();
    for existing in [None, Some("CONFIRMED006")] {
        let mut tags = sample();
        put(&mut tags, "ALBUM", "Ombra Intl 006");
        tags.remove("CATALOGNUMBER");
        if let Some(code) = existing {
            put(&mut tags, "CATALOGNUMBER", code);
        }
        let mut tracks = vec![Track {
            path: r.source.clone(),
            tags,
            duration: 0.15,
        }];
        let plan = prepare_tags(
            &r,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        let code = existing.unwrap_or("OMBRA INTL 006");
        assert_eq!(plan.album, "Ombra International 006");
        assert_eq!(plan.catalog, code);
        assert_eq!(
            plan.directory,
            PathBuf::from(format!("Romy/Ombra International 006 [DE] [{code}]"))
        );
        assert_eq!(
            value(&tracks[0].tags, "ALBUM"),
            format!("Ombra International 006 [DE] [{code}]")
        );
        assert_eq!(value(&tracks[0].tags, "CATALOGNUMBER"), code);
    }
}

#[test]
fn processing_removes_rejected_existing_genres_instead_of_leaving_old_tags() {
    let mut r = release(PathBuf::from("unused.flac"));
    r.genre = "123;80s;Romy;x;Other Band".into();
    let mut tags = sample();
    put(&mut tags, "GENRE", "2026;TECHNO2;Romy;!");
    put(&mut tags, "PERFORMER", "Other Band");
    let mut tracks = vec![Track {
        path: r.source.clone(),
        tags,
        duration: 0.15,
    }];
    prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert!(!tracks[0].tags.contains_key("GENRE"));
    r.genre = "AMBIENT;ambient;post-punk".into();
    prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    assert_eq!(tracks[0].tags["GENRE"], ["Ambient", "Post-Punk"]);
}

#[test]
fn reserved_publication_supports_plain_rename_without_overwriting_any_destination() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("staging");
    let destination = root.path().join("album");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("track.flac"), b"verified payload").unwrap();
    fs::write(source.join(".bcfunl-release.json"), b"manifest").unwrap();
    publish_reserved(&source, &destination).unwrap();
    assert!(!source.exists());
    assert_eq!(
        fs::read(destination.join("track.flac")).unwrap(),
        b"verified payload"
    );
    fs::create_dir(&source).unwrap();
    fs::write(source.join("track.flac"), b"replacement").unwrap();
    assert!(publish_reserved(&source, &destination).is_err());
    assert_eq!(
        fs::read(destination.join("track.flac")).unwrap(),
        b"verified payload"
    );
    let empty = root.path().join("existing-empty");
    fs::create_dir(&empty).unwrap();
    assert!(publish_reserved(&source, &empty).is_err());
    assert!(empty.is_dir());
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&destination, &alias).unwrap();
    assert!(publish_reserved(&source, &alias).is_err());
    let failed = root.path().join("failed");
    assert!(publish_reserved(&root.path().join("missing"), &failed).is_err());
    assert!(!failed.exists());
    assert!(source.join("track.flac").exists());
}

#[test]
fn existing_scan_ignores_unrelated_temporary_folders_but_keeps_real_duplicate_review() {
    let root = tempfile::tempdir().unwrap();
    let (_fixture, path) = fixture("flac");
    tagging::write(&path, &sample()).unwrap();
    let r = release(path.clone());
    let mut tracks = vec![Track {
        path,
        tags: sample(),
        duration: 0.15,
    }];
    let plan = prepare_tags(
        &r,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    // An unrelated broken link is never probed as a candidate artist directory.
    std::os::unix::fs::symlink(
        root.path().join("gone"),
        root.path().join(".bcfunl-process-other"),
    )
    .unwrap();
    assert!(existing(root.path(), &plan, &tracks).unwrap().is_none());
    for name in ["Proof [GB] [CAT01]", "Proof [US] [OLD01]"] {
        let folder = root.path().join("Romy").join(name);
        fs::create_dir_all(&folder).unwrap();
        fs::copy(&tracks[0].path, folder.join("01.flac")).unwrap();
    }
    assert!(
        existing(root.path(), &plan, &tracks)
            .unwrap_err()
            .to_string()
            .contains("Multiple verified server copies")
    );
}

#[test]
fn limited_process_cleanup_does_not_walk_or_clean_unrelated_history() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let server = root.path().join("server");
    fs::create_dir_all(input.join("fixture/Old")).unwrap();
    let old = input.join("fixture/Old/old.flac");
    fs::write(&old, b"retain old payload").unwrap();
    let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,purchase_date) VALUES('fixture','new','New','New','unused','complete','missing.zip','2026-10-30')",&[]).unwrap();
    db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,purchase_date,process_date,processed_path) VALUES('fixture','old','Old','Old','unused','complete',?,'2020-01-01','success','unavailable/server')",&[old.to_str().unwrap()]).unwrap();
    let args = Args::try_parse_from([
        "bcfunl",
        "--process",
        "--offline",
        "--clean",
        "--count",
        "1",
        "--timings",
        "--user",
        "fixture",
        "--output",
        input.to_str().unwrap(),
        "--serverdir",
        server.to_str().unwrap(),
    ])
    .unwrap();
    let error = run(&args).unwrap_err().to_string();
    assert!(error.contains("1 releases failed processing"), "{error}");
    assert_eq!(fs::read(old).unwrap(), b"retain old payload");
    let rows = db
        .query(
            "SELECT item_key FROM releases WHERE process_error IS NOT NULL",
            &[],
        )
        .unwrap();
    assert_eq!(rows[0][0].as_deref(), Some("new"));
    assert_eq!(rows.len(), 1);
}

#[test]
fn audio_read_retry_is_bounded_and_preserves_success_or_failure() {
    let mut attempts = 0;
    let result = retry_audio(Path::new("fixture.flac"), || {
        attempts += 1;
        if attempts == 1 {
            anyhow::bail!("transient read");
        }
        Ok("verified")
    })
    .unwrap();
    assert_eq!(result, "verified");
    assert_eq!(attempts, 2);
    attempts = 0;
    let error = retry_audio::<()>(Path::new("fixture.flac"), || {
        attempts += 1;
        anyhow::bail!("persistent decoder error");
    })
    .unwrap_err();
    assert_eq!(attempts, 2);
    assert!(format!("{error:#}").contains("persistent decoder error"));
    assert!(format!("{error:#}").contains("after 2 attempts"));
    attempts = 0;
    retry_audio(Path::new("fixture.flac"), || {
        attempts += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(attempts, 1);
}

#[test]
fn fast_flac_tag_readback_matches_full_reader() {
    let (_root, path) = fixture("flac");
    let mut tags = sample();
    tags.insert("GENRE".into(), vec!["Ambient;Electronic".into()]);
    tags.insert("PERFORMER".into(), vec!["Romy".into(), "Peaches".into()]);
    tagging::write(&path, &tags).unwrap();
    assert_eq!(
        tagging::read_tags(&path).unwrap(),
        tagging::read(&path).unwrap().0
    );
}

#[test]
fn native_decoding_matches_independent_lossless_pcm_and_rejects_truncation() {
    for ext in ["flac", "wav", "aiff", "m4a"] {
        let (_dir, path) = fixture(ext);
        let reference = command(
            Command::new("ffmpeg")
                .args(["-v", "error", "-xerror", "-nostdin", "-i"])
                .arg(&path)
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
        .unwrap();
        assert!(
            pcm_hash(&path).unwrap().ends_with(reference.trim()),
            "{ext}"
        );
        if ext == "flac" {
            let len = fs::metadata(&path).unwrap().len();
            OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(len - 100)
                .unwrap();
            assert!(pcm_hash(&path).is_err());
        }
    }
}

#[test]
fn native_tags_preserve_unknown_fields_artwork_and_remove_discarded_genres() {
    for ext in ["flac", "wav", "mp3", "aiff", "m4a", "aac-hi", "ogg"] {
        let (_dir, path) = fixture(ext);
        let mut tags = sample();
        tags.insert(
            "BCFUNL_TEST_UNKNOWN".into(),
            vec!["Keep me".into(), "Keep me too".into()],
        );
        tags.insert("GENRE".into(), vec!["123".into()]);
        tagging::write(&path, &tags).unwrap();
        let mut replacement = sample();
        replacement.insert("ARTIST".into(), vec!["First".into(), "Second".into()]);
        tagging::write(&path, &replacement).unwrap();
        let read = tagging::read_tags(&path).unwrap();
        assert!(!read.contains_key("GENRE"), "{ext}: {read:?}");
        assert_eq!(
            read["BCFUNL_TEST_UNKNOWN"],
            ["Keep me", "Keep me too"],
            "{ext}"
        );
        assert_eq!(read["ARTIST"], ["First", "Second"], "{ext}");
    }
}

#[test]
fn multiple_tracks_and_releases_share_bounded_native_workers() {
    let (input, path) = fixture("flac");
    let mut paths = Vec::new();
    for n in 1..=8 {
        let track = input.path().join(format!("track{n}.flac"));
        fs::copy(&path, &track).unwrap();
        let mut tags = sample();
        put(&mut tags, "TRACKNUMBER", n.to_string());
        put(&mut tags, "TITLE", format!("Track {n}"));
        tagging::write(&track, &tags).unwrap();
        paths.push(track);
    }
    assert!(file_pool().unwrap().current_num_threads() <= 8);
    thread::scope(|scope| {
        let jobs: Vec<_> = (0..2)
            .map(|_| {
                let paths = &paths;
                let path = &path;
                scope.spawn(move || {
                    let server = tempfile::tempdir().unwrap();
                    let mut tracks = tracks(paths).unwrap();
                    let plan = prepare_tags(
                        &release(path.clone()),
                        &mut tracks,
                        &template::Template::new(template::DEFAULT).unwrap(),
                    )
                    .unwrap();
                    file_pool().unwrap().install(|| {
                        tracks
                            .par_iter()
                            .zip(&plan.files)
                            .for_each(|(track, relative)| {
                                let dest = server.path().join(relative);
                                fs::create_dir_all(dest.parent().unwrap()).unwrap();
                                fs::copy(&track.path, &dest).unwrap();
                                tagging::write(&dest, &track.tags).unwrap();
                                verify_tags(&track.tags, &dest).unwrap();
                                assert_eq!(
                                    pcm_hash(&track.path).unwrap(),
                                    pcm_hash(&dest).unwrap()
                                );
                            })
                    });
                })
            })
            .collect();
        for job in jobs {
            job.join().unwrap();
        }
    });
}

#[test]
#[ignore = "manual optimized comparison; requires FFmpeg to generate reference audio"]
fn benchmark_native_decoding_against_subprocess_reference() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("minute.flac");
    command(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=60",
                "-ar",
                "96000",
                "-ac",
                "2",
                "-sample_fmt",
                "s32",
            ])
            .arg(&path),
    )
    .unwrap();
    let start = std::time::Instant::now();
    let native = pcm_hash(&path).unwrap();
    let native_time = start.elapsed();
    let start = std::time::Instant::now();
    command(
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
            .arg(&path),
    )
    .unwrap();
    let reference = command(
        Command::new("ffmpeg")
            .args(["-v", "error", "-xerror", "-nostdin", "-i"])
            .arg(&path)
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
    .unwrap();
    let reference_time = start.elapsed();
    assert!(native.ends_with(reference.trim()));
    eprintln!(
        "60s stereo 96kHz FLAC: native={:.3}s, previous subprocess path={:.3}s",
        native_time.as_secs_f64(),
        reference_time.as_secs_f64()
    );
}

#[test]
fn legacy_riff_and_aiff_text_tags_are_read_and_reconciled() {
    for ext in ["wav", "aiff"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("legacy.{ext}"));
        command(
            Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=duration=0.1",
                    "-metadata",
                    "title=Old Title",
                    "-metadata",
                    "artist=Old Artist",
                ])
                .arg(&path),
        )
        .unwrap();
        let (mut tags, _) = tagging::read(&path).unwrap();
        assert_eq!(tags["TITLE"], ["Old Title"], "{ext}");
        put(&mut tags, "TITLE", "New Title");
        tagging::write(&path, &tags).unwrap();
        assert_eq!(tagging::read(&path).unwrap().0["TITLE"], ["New Title"]);
        use lofty::file::TaggedFileExt;
        for tag in lofty::read_from_path(&path).unwrap().tags() {
            use lofty::prelude::{Accessor, ItemKey};
            if let Some(title) = tag.title() {
                assert_eq!(title, "New Title", "{ext}");
            }
            assert!(
                tag.get_strings(ItemKey::TrackTitle)
                    .all(|title| title == "New Title")
            );
        }
    }
}

#[test]
fn native_high_resolution_pcm_matches_independent_decoder() {
    for (ext, codec) in [
        ("flac", "flac"),
        ("wav", "pcm_s24le"),
        ("wav", "pcm_f64le"),
        ("aiff", "pcm_s24be"),
        ("aiff", "pcm_f32be"),
        ("m4a", "alac"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(format!("audio.{ext}"));
        command(
            Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=duration=0.1",
                    "-ar",
                    "96000",
                    "-ac",
                    "2",
                    "-c:a",
                    codec,
                ])
                .arg(&path),
        )
        .unwrap();
        let reference = command(
            Command::new("ffmpeg")
                .args(["-v", "error", "-xerror", "-nostdin", "-i"])
                .arg(&path)
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
        .unwrap();
        assert!(
            pcm_hash(&path).unwrap().ends_with(reference.trim()),
            "{ext}: {codec}"
        );
    }
}

#[test]
fn native_multivalues_expand_semicolons_and_preserve_multiline_lyrics() {
    for ext in ["flac", "wav", "mp3", "aiff", "m4a", "aac-hi", "ogg"] {
        let (_dir, path) = fixture(ext);
        let mut tags = sample();
        for key in [
            "ARTIST",
            "TRACKARTIST",
            "ALBUMARTIST",
            "ARTISTSORT",
            "ALBUMARTISTSORT",
            "PERFORMER",
            "COMPOSER",
            "CONDUCTOR",
            "REMIXER",
            "PRODUCER",
        ] {
            tags.insert(key.into(), vec![" First ; Second; ;First ".into()]);
        }
        tags.insert("GENRE".into(), vec!["Ambient; Electronic; ".into()]);
        tags.insert("TITLE".into(), vec!["One; Complete Title".into()]);
        tags.insert(
            "UNSYNCEDLYRICS".into(),
            vec!["First line\r\nSecond line; lyric text\r\n".into()],
        );
        let before = pcm_hash(&path).unwrap();
        tagging::write(&path, &tags).unwrap();
        let read = tagging::read_tags(&path).unwrap();
        for key in [
            "ARTIST",
            "TRACKARTIST",
            "ALBUMARTIST",
            "ARTISTSORT",
            "ALBUMARTISTSORT",
            "PERFORMER",
            "COMPOSER",
            "CONDUCTOR",
            "REMIXER",
            "PRODUCER",
        ] {
            assert_eq!(read[key], ["First", "Second"], "{ext}: {key}");
        }
        assert_eq!(read["GENRE"], ["Ambient", "Electronic"], "{ext}");
        assert_eq!(read["TITLE"], tags["TITLE"], "{ext}");
        assert_eq!(read["UNSYNCEDLYRICS"], tags["UNSYNCEDLYRICS"], "{ext}");
        assert_eq!(pcm_hash(&path).unwrap(), before, "{ext}");
    }
    assert_eq!(credits::artist_names("First; Second;"), ["First", "Second"]);
}

#[test]
fn comma_performer_lists_expand_without_splitting_sort_names_or_mapped_bands() {
    let artists = "Amevicious, Phred Noir, Bardamu, Nici Palm, Rødel, Akrav";
    let expected = [
        "Amevicious",
        "Phred Noir",
        "Bardamu",
        "Nici Palm",
        "Rødel",
        "Akrav",
    ];
    assert_eq!(credits::artist_names(artists), expected);
    for ext in ["flac", "wav", "mp3", "aiff", "m4a", "ogg"] {
        let (_dir, path) = fixture(ext);
        let mut tags = sample();
        for key in ["ARTIST", "TRACKARTIST", "PERFORMER", "ALBUMARTIST"] {
            put(&mut tags, key, artists);
        }
        put(&mut tags, "ARTISTSORT", "Madley Croft, Romy; Phred Noir");
        put(&mut tags, "ALBUMARTISTSORT", "Madley Croft, Romy");
        put(&mut tags, "TITLE", "One, Complete Title");
        tagging::write(&path, &tags).unwrap();
        let read = tagging::read_tags(&path).unwrap();
        for key in ["ARTIST", "TRACKARTIST", "PERFORMER", "ALBUMARTIST"] {
            assert_eq!(read[key], expected, "{ext}: {key}");
        }
        assert_eq!(read["ARTISTSORT"], ["Madley Croft, Romy", "Phred Noir"]);
        assert_eq!(read["ALBUMARTISTSORT"], ["Madley Croft, Romy"]);
        assert_eq!(read["TITLE"], ["One, Complete Title"]);
        let mut tracks = tracks(std::slice::from_ref(&path)).unwrap();
        let plan = prepare_tags(
            &release(path),
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        )
        .unwrap();
        assert_eq!(plan.artist, "Amevicious");
        assert_eq!(tracks[0].tags["TRACKARTIST"], expected);
        assert_eq!(tracks[0].tags["PERFORMER"], expected);
    }
    assert_eq!(
        credits::artist_names("Earth, Wind & Fire"),
        ["Earth, Wind & Fire"]
    );
}

#[test]
fn six_strings_track_credits_do_not_inherit_other_album_performers() {
    let (_dir, path) = fixture("flac");
    let mut release = release(path.clone());
    release.artist = "Amevicious, Phred Noir, Bardamu, Nici Palm, Rødel, Akrav".into();
    release.album = "Six Strings In The Devil's Sorrow".into();
    let expected = [
        ("The Atonement", vec!["Amevicious", "Phred Noir"]),
        ("Strange People", vec!["Bardamu"]),
        ("Functional Freeze", vec!["Nici Palm", "Rødel"]),
        ("Satanisch", vec!["Akrav", "Solas"]),
    ];
    let mut tracks: Vec<_> = expected
        .iter()
        .enumerate()
        .map(|(n, (title, names))| {
            let mut tags = Tags::new();
            put(&mut tags, "ARTIST", names.join(", "));
            put(&mut tags, "ALBUMARTIST", &release.artist);
            put(&mut tags, "ALBUM", &release.album);
            put(&mut tags, "TITLE", title);
            put(&mut tags, "TRACKNUMBER", n + 1);
            Track {
                path: path.clone(),
                tags,
                duration: 0.15,
            }
        })
        .collect();
    prepare_tags(
        &release,
        &mut tracks,
        &template::Template::new(template::DEFAULT).unwrap(),
    )
    .unwrap();
    for (track, (title, names)) in tracks.iter().zip(expected) {
        assert_eq!(track.tags["TRACKARTIST"], names, "{title}");
        assert_eq!(track.tags["PERFORMER"], names, "{title}");
        tagging::write(&path, &track.tags).unwrap();
        verify_tags(&track.tags, &path).unwrap();
    }
}

#[test]
fn discogs_enrichment_precedes_template_and_publishes_authoritative_genres() {
    let (input, path) = fixture("flac");
    let mut tags = sample();
    tags.remove("CATALOGNUMBER");
    put(&mut tags, "GENRE", "Bandcamp Junk");
    tagging::write(&path, &tags).unwrap();
    let server = tempfile::tempdir().unwrap();
    let lookup = discogs::fixture();
    let (destination, plan, skipped) = process_one(
        &release(path),
        input.path(),
        server.path(),
        &template::Template::new(template::DEFAULT).unwrap(),
        None,
        Some(&lookup),
        false,
    )
    .unwrap();
    assert!(!skipped);
    assert_eq!(plan.catalog, "DG001");
    assert!(destination.to_string_lossy().contains("[DG001]"));
    let published = tracks(&collect(&destination).unwrap()).unwrap();
    assert_eq!(published[0].tags["GENRE"], ["Electronic", "EBM"]);
    assert_eq!(published[0].tags["DISCOGSID"], ["1"]);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(destination.join(".bcfunl-release.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["discogs_metadata"]["country_iso"], "GB");
}

#[test]
#[ignore = "manual live Discogs API check; requires credentials and network"]
fn live_discogs_metadata_smoke() {
    let root = tempfile::tempdir().unwrap();
    let lookup = discogs::Lookup::from_env(root.path())
        .unwrap()
        .expect("Discogs token required");
    let mut release = release("unused.flac".into());
    release.artist = "David Holmes".into();
    release.album = "Blind On A Galloping Horse (Remixes Vol. 2)".into();
    release.country = "GB".into();
    let result = lookup
        .find(&release, &[])
        .expect("Live Discogs request failed");
    eprintln!(
        "Live Discogs artist/title/country match: {}",
        result
            .as_ref()
            .map(|m| m.evidence.to_string())
            .unwrap_or_else(|| "unresolved".into())
    );
}

#[test]
fn unmatched_processing_keeps_only_recognized_source_genres() {
    let (input, path) = fixture("flac");
    let mut tags = sample();
    put(
        &mut tags,
        "GENRE",
        "Some Made Up Genre Phrase;EBM - electronic body music",
    );
    tagging::write(&path, &tags).unwrap();
    let mut release = release(path);
    release.genre = "Another Unrecognized Phrase".into();
    let server = tempfile::tempdir().unwrap();
    let (destination, _, _) = process_one(
        &release,
        input.path(),
        server.path(),
        &template::Template::new(template::DEFAULT).unwrap(),
        None,
        None,
        false,
    )
    .unwrap();
    let files = tracks(&collect(&destination).unwrap()).unwrap();
    assert_eq!(files[0].tags["GENRE"], ["EBM", "Electronic"]);
}

#[test]
fn album_catalog_track_decorations_work_across_labels_and_preserve_known_catalogs() {
    for code in [
        "Ombrantl033",
        "ABC033",
        "ABC 033",
        "ABC-033",
        "ABC - 033",
        "XYZ009",
    ] {
        for existing in [None, Some("CONFIRMED001")] {
            let mut release = release("unused.flac".into());
            release.artist = "SLF IMG".into();
            release.album = "Chiodi EP".into();
            release.genre.clear();
            release.country = "DE".into();
            let mut tags = sample();
            put(&mut tags, "ALBUMARTIST", "SLF IMG");
            put(&mut tags, "ARTIST", "SLF IMG");
            put(&mut tags, "ALBUM", "Chiodi EP");
            put(&mut tags, "TITLE", format!("Sex [Chiodi EP-{code}]"));
            tags.remove("CATALOGNUMBER");
            if let Some(existing) = existing {
                put(&mut tags, "CATALOGNUMBER", existing);
            }
            let mut tracks = vec![Track {
                path: release.source.clone(),
                tags,
                duration: 0.15,
            }];
            let plan = prepare_tags(
                &release,
                &mut tracks,
                &template::Template::new(template::DEFAULT).unwrap(),
            )
            .unwrap();
            let expected = existing
                .map(str::to_owned)
                .unwrap_or_else(|| code.to_uppercase());
            assert_eq!(plan.catalog, expected);
            assert_eq!(tracks[0].tags["TITLE"], ["Sex"]);
            assert_eq!(tracks[0].tags["CATALOGNUMBER"], [expected.as_str()]);
            assert_eq!(
                plan.directory,
                PathBuf::from(format!("SLF IMG/Chiodi EP [DE] [{expected}]"))
            );
            assert_eq!(plan.files, [PathBuf::from("01-01 - Sex.flac")]);
        }
    }
    for raw in [
        "Sex (2026 Remaster)",
        "Sex [Chiodi EP - Extended Mix]",
        "Sex [Other Album-ABC033]",
        "Sex [Chiodi EP-2026]",
        "Sex [Chiodi EP-ABC033 Remix]",
    ] {
        assert_eq!(
            crate::sanitizing::track_title(raw, "Chiodi EP").1,
            None,
            "{raw}"
        );
    }
    assert_eq!(
        crate::sanitizing::track_title("Sex (chiodi ep : ABC-033)", "Chiodi EP"),
        ("Sex".into(), Some("ABC-033".into()))
    );
}

#[test]
fn track_catalog_inference_accepts_spacing_variants_but_rejects_conflicting_codes() {
    for (codes, valid) in [
        (vec!["ABC033", "ABC 033", "ABC-033", "ABC - 033"], true),
        (vec!["ABC033", "ABC034"], false),
    ] {
        let mut release = release("unused.flac".into());
        release.album = "Generic EP".into();
        let mut tracks: Vec<_> = codes
            .iter()
            .enumerate()
            .map(|(i, code)| {
                let mut tags = sample();
                tags.remove("CATALOGNUMBER");
                put(&mut tags, "ALBUM", "Generic EP");
                put(
                    &mut tags,
                    "TITLE",
                    format!("Track {} [Generic EP-{code}]", i + 1),
                );
                put(&mut tags, "TRACKNUMBER", i + 1);
                Track {
                    path: release.source.clone(),
                    tags,
                    duration: 0.15,
                }
            })
            .collect();
        let result = prepare_tags(
            &release,
            &mut tracks,
            &template::Template::new(template::DEFAULT).unwrap(),
        );
        if valid {
            let plan = result.unwrap();
            assert_eq!(plan.catalog, "ABC033");
            assert_eq!(tracks[0].tags["TITLE"], ["Track 1"]);
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn existing_audio_matches_despite_legacy_title_damage_but_requires_unique_positions() {
    let (_input, path) = fixture("flac");
    let mut original = sample();
    put(&mut original, "TITLE", "Gholü (Nicolás Remix)");
    tagging::write(&path, &original).unwrap();
    let server = tempfile::tempdir().unwrap();
    let destination = server.path().join("01 - Ghol_.flac");
    fs::copy(&path, &destination).unwrap();
    let mut legacy = original.clone();
    put(&mut legacy, "TITLE", "Ghol? (Nicol?S Remix)");
    tagging::write(&destination, &legacy).unwrap();
    let before = fs::read(&destination).unwrap();
    let source = tracks(&[path]).unwrap();
    assert!(matching_release(&source, server.path()).unwrap());
    assert_eq!(fs::read(&destination).unwrap(), before);
    put(&mut legacy, "DISCNUMBER", "2");
    tagging::write(&destination, &legacy).unwrap();
    assert!(!matching_release(&source, server.path()).unwrap());
    put(&mut legacy, "DISCNUMBER", "1");
    tagging::write(&destination, &legacy).unwrap();
    fs::copy(&destination, server.path().join("duplicate.flac")).unwrap();
    assert!(!matching_release(&source, server.path()).unwrap());
}

#[test]
#[ignore = "read-only acceptance against an existing archive and server release"]
fn existing_release_read_only_acceptance() {
    let archive = std::env::var_os("BCFUNL_TEST_ARCHIVE").expect("BCFUNL_TEST_ARCHIVE");
    let destination = std::env::var_os("BCFUNL_TEST_DESTINATION").expect("BCFUNL_TEST_DESTINATION");
    let work = tempfile::tempdir().unwrap();
    let source = tracks(&extract(Path::new(&archive), work.path()).unwrap()).unwrap();
    assert!(matching_release(&source, Path::new(&destination)).unwrap());
}
