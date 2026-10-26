//! Offline artist-level evidence, used only after release/embedded genres are exhausted.
use super::{Track, value};
use crate::db::Database;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::Path;

pub(super) struct Library {
    db: Database,
    source: Value,
}
impl Library {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let db = Database::read_only(path.to_str().context("Genre library path is not UTF-8")?)
            .context("Open genre research library")?;
        let rows = db.query(
            "SELECT source_file,source_sha256 FROM resource_import WHERE dataset='masters'",
            &[],
        )?;
        let row = rows
            .first()
            .context("Genre library has no master snapshot provenance")?;
        let source = json!({"dataset":"discogs-masters","file":row[0],"sha256":row[1]});
        // Validate the lookup schema even when the current batch needs no fallback.
        db.query("SELECT a.id,g.genre,g.master_count FROM artists a JOIN artist_genres g ON a.id=g.artist_id LIMIT 0", &[])?;
        Ok(Self { db, source })
    }
    pub(super) fn apply(&self, artist: &str, tracks: &mut [Track]) -> Result<Option<Value>> {
        if !tracks
            .iter()
            .any(|t| value(&t.tags, "GENRE").trim().is_empty())
        {
            return Ok(None);
        }
        let matches = self.db.query(
            "SELECT id,name FROM artists WHERE name=? COLLATE NOCASE LIMIT 2",
            &[artist],
        )?;
        if matches.len() != 1 {
            eprintln!(
                "  Genre fallback unresolved: {} exact artist matches for {artist}",
                matches.len()
            );
            return Ok(None);
        }
        let id = matches[0][0]
            .as_deref()
            .context("Missing research artist ID")?;
        // Use the five strongest classifications rather than an artist's whole career vocabulary.
        let evidence=self.db.query("SELECT genre,master_count FROM artist_genres WHERE artist_id=? ORDER BY master_count DESC,genre COLLATE NOCASE LIMIT 5", &[id])?;
        let values: Vec<_> = evidence.iter().filter_map(|row| row[0].clone()).collect();
        let mut genres = vcw_metadata::Genres::builtin().normalize_all(&values);
        genres.truncate(5);
        if genres.is_empty() {
            eprintln!("  Genre fallback unresolved: no master evidence for {artist}");
            return Ok(None);
        }
        let provenance=json!({"kind":"artist-default","artist_id":id,"artist":matches[0][1],"source":self.source}).to_string();
        let mut affected = Vec::new();
        for track in tracks {
            if value(&track.tags, "GENRE").trim().is_empty() {
                track.tags.insert("GENRE".into(), genres.clone());
                track
                    .tags
                    .insert("BCFUNL_GENRE_SOURCE".into(), vec![provenance.clone()]);
                affected.push(json!({"disc":value(&track.tags,"DISCNUMBER"),"track":value(&track.tags,"TRACKNUMBER")}));
            }
        }
        eprintln!(
            "  Assigned artist-default genres to {} tracks: {}",
            affected.len(),
            genres.join("; ")
        );
        Ok(Some(
            json!({"artist_id":id,"artist":matches[0][1],"genres":genres,"source":self.source,"evidence":evidence,"tracks":affected}),
        ))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs, path::PathBuf};
    pub(crate) fn fixture_library(path: &Path) {
        let db = Database::connect(path.to_str().unwrap()).unwrap();
        for sql in [
            "CREATE TABLE artists(id INTEGER,name TEXT)",
            "CREATE TABLE artist_genres(artist_id INTEGER,genre TEXT,master_count INTEGER)",
            "CREATE TABLE resource_import(dataset TEXT,source_file TEXT,source_sha256 TEXT)",
            "INSERT INTO artists VALUES(1,'Romy')",
            "INSERT INTO resource_import VALUES('masters','discogs_20261001_masters.xml.gz','fixture-checksum')",
        ] {
            db.query(sql, &[]).unwrap();
        }
        for (genre, count) in [
            ("Electronic", 100),
            ("Ambient", 90),
            ("Techno", 80),
            ("IDM", 70),
            ("Drone", 60),
            ("Leftfield", 50),
        ] {
            db.query(
                "INSERT INTO artist_genres VALUES(1,?,?)",
                &[genre, &count.to_string()],
            )
            .unwrap();
        }
    }
    fn track(genre: Option<&str>) -> Track {
        let mut tags = BTreeMap::new();
        if let Some(genre) = genre {
            tags.insert("GENRE".into(), vec![genre.into()]);
        }
        Track {
            path: PathBuf::from("fixture.flac"),
            tags,
            duration: 1.0,
        }
    }
    #[test]
    fn fallback_is_ranked_track_local_provenanced_and_read_only() {
        let root = super::super::tempfile::tempdir().unwrap();
        let path = root.path().join("library.sqlite3");
        fixture_library(&path);
        let before = fs::read(&path).unwrap();
        let library = Library::open(&path).unwrap();
        let mut tracks = vec![track(Some("Rock")), track(None)];
        let audit = library.apply("rOmY", &mut tracks).unwrap().unwrap();
        assert_eq!(tracks[0].tags["GENRE"], ["Rock"]);
        assert!(!tracks[0].tags.contains_key("BCFUNL_GENRE_SOURCE"));
        assert_eq!(
            tracks[1].tags["GENRE"],
            ["Electronic", "Ambient", "Techno", "IDM", "Drone"]
        );
        assert_eq!(audit["tracks"].as_array().unwrap().len(), 1);
        assert_eq!(audit["artist_id"], "1");
        assert!(
            tracks[1].tags["BCFUNL_GENRE_SOURCE"][0].contains("discogs_20261001_masters.xml.gz")
        );
        assert!(library.apply("Romy", &mut tracks).unwrap().is_none());
        assert!(library.db.query("DELETE FROM artists", &[]).is_err());
        drop(library);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    #[test]
    fn missing_or_ambiguous_artist_is_not_guessed_and_missing_library_is_not_created() {
        let root = super::super::tempfile::tempdir().unwrap();
        let missing = root.path().join("missing.sqlite3");
        assert!(Library::open(&missing).is_err());
        assert!(!missing.exists());
        let path = root.path().join("library.sqlite3");
        fixture_library(&path);
        let db = Database::connect(path.to_str().unwrap()).unwrap();
        db.query("INSERT INTO artists VALUES(2,'ROMY')", &[])
            .unwrap();
        drop(db);
        let library = Library::open(&path).unwrap();
        let mut tracks = vec![track(None)];
        for name in ["Romy", "Unknown Artist", "Rom"] {
            assert!(library.apply(name, &mut tracks).unwrap().is_none());
            assert!(!tracks[0].tags.contains_key("GENRE"));
        }
    }
}
