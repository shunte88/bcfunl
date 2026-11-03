//! Discogs provider/client reuse from VCW, with bcfunl's existing HTTP stack.
use super::{Release, Track, common, value};
use crate::shutdown;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
use vcw_metadata::{
    Cancel, Client, Clock, Discogs, Provider, ProviderId, Query, Request, Response, SystemClock,
    Token, Transport, TransportError,
};

#[derive(Debug)]
struct Http {
    client: reqwest::Client,
    runtime: Option<tokio::runtime::Runtime>,
}
impl Drop for Http {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
impl Http {
    fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            runtime: Some(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()?,
            ),
        })
    }
}
impl Transport for Http {
    fn name(&self) -> &'static str {
        "bcfunl-http"
    }
    fn send(&self, request: &Request) -> std::result::Result<Response, TransportError> {
        let run = || -> Result<Response> {
            shutdown::check()?;
            let url = reqwest::Url::parse(&request.url)?;
            ensure!(
                url.scheme() == "https"
                    && url.host_str() == Some("api.discogs.com")
                    && url.port_or_known_default() == Some(443)
                    && url.username().is_empty()
                    && url.password().is_none()
                    && request.body.is_none(),
                "Unexpected Discogs origin/method"
            );
            self.runtime
                .as_ref()
                .context("Discogs runtime stopped")?
                .block_on(shutdown::until(async {
                    let mut get = self.client.get(url).timeout(request.timeout);
                    for header in &request.headers {
                        let mut value = reqwest::header::HeaderValue::from_str(&header.value)?;
                        value.set_sensitive(header.is_sensitive());
                        get = get.header(&header.name, value);
                    }
                    let mut response = get.send().await.map_err(|e| e.without_url())?;
                    let status = response.status().as_u16();
                    let headers = response
                        .headers()
                        .iter()
                        .filter_map(|(k, v)| {
                            v.to_str()
                                .ok()
                                .map(|v| vcw_metadata::net::Header::new(k.as_str(), v))
                        })
                        .collect();
                    const MAX: usize = 8 * 1024 * 1024;
                    ensure!(
                        response.content_length().is_none_or(|n| n <= MAX as u64),
                        "Discogs response too large"
                    );
                    let mut body = Vec::new();
                    while let Some(chunk) = response.chunk().await.map_err(|e| e.without_url())? {
                        ensure!(
                            body.len() + chunk.len() <= MAX,
                            "Discogs response too large"
                        );
                        body.extend_from_slice(&chunk);
                    }
                    Ok(Response {
                        status,
                        headers,
                        body,
                    })
                }))?
        };
        run().map_err(|e| TransportError::Unreachable(format!("{e:#}")))
    }
}
#[derive(Debug)]
struct SignalClock {
    clock: SystemClock,
    cancel: Cancel,
}
impl Clock for SignalClock {
    fn now_millis(&self) -> u64 {
        self.clock.now_millis()
    }
    fn sleep(&self, duration: std::time::Duration) {
        if shutdown::pause(duration).is_err() {
            self.cancel.cancel();
        }
    }
}

pub(super) struct Lookup {
    provider: Discogs,
    cancel: Cancel,
}
#[derive(Clone, Debug)]
pub(super) struct Match {
    pub catalog: Option<String>,
    pub genres: Vec<String>,
    pub evidence: Value,
    pub ids: Vec<String>,
}
fn country(value: &str) -> String {
    if value.eq_ignore_ascii_case("EU") || value.eq_ignore_ascii_case("Europe") {
        "EU".into()
    } else {
        crate::metadata::country(value).unwrap_or_default().into()
    }
}
fn country_matches(provider: &str, source: &str) -> bool {
    provider.trim().eq_ignore_ascii_case("Worldwide")
        || (!source.is_empty()
            && (country(provider) == source
                || provider
                    .split(['&', ',', '/', ';'])
                    .any(|part| country(part.trim()) == source)))
}
// Edition notes differ across stores; retain them in tags, not lookup identity.
fn search_title(raw: &str) -> String {
    fn edition(note: &str) -> bool {
        let note = note
            .to_ascii_lowercase()
            .replace("re-issued", "reissued")
            .replace("re-mastered", "remastered");
        let words: Vec<_> = note
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let mut year = false;
        let mut kind = false;
        for word in words {
            if word.len() == 4
                && word
                    .parse::<u16>()
                    .is_ok_and(|n| (1900..=2099).contains(&n))
            {
                year = true;
            } else {
                match word.to_ascii_lowercase().as_str() {
                    "remaster" | "remastered" | "reissue" | "reissued" | "rm" | "re" => kind = true,
                    "digital" | "digitally" | "edition" | "version" | "in" => {}
                    _ => return false,
                }
            }
        }
        year && kind
    }
    let mut title = raw.trim();
    loop {
        let mut cut = None;
        for (open, close) in [('(', ')'), ('[', ']')] {
            if title.ends_with(close)
                && let Some(pos) = title.rfind(open)
                && edition(&title[pos + 1..title.len() - 1])
            {
                cut = Some(pos);
            }
        }
        if cut.is_none() {
            for (pos, c) in title.char_indices() {
                if c.is_whitespace() && edition(title[pos..].trim()) {
                    cut = Some(pos);
                    break;
                }
            }
        }
        let Some(pos) = cut else { break };
        let base = title[..pos]
            .trim_end()
            .trim_end_matches(['-', ':', '–', '—'])
            .trim_end();
        if base.is_empty() {
            break;
        }
        title = base;
    }
    title.to_owned()
}

fn same(a: &str, b: &str) -> bool {
    !super::key(a).is_empty() && super::key(&search_title(a)) == super::key(&search_title(b))
}
fn same_artist(a: &str, b: &str) -> bool {
    let clean = |name: &str| {
        name.trim()
            .trim_end_matches('*')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let a = clean(a);
    !a.is_empty() && a == clean(b)
}
// Credit expansions can include guests; only reciprocal map entries establish aliases.
fn artist_aliases(artist: &str) -> Vec<String> {
    let mut aliases = vec![artist.to_owned()];
    if let Some(names) =
        crate::sanitizing::mapped_artists(artist.trim().trim_end_matches('*').trim_end())
    {
        for name in names {
            if crate::sanitizing::mapped_artists(name)
                .is_some_and(|reverse| reverse.iter().any(|original| same_artist(original, artist)))
                && !aliases.iter().any(|alias| same_artist(alias, name))
            {
                aliases.push(name.clone());
            }
        }
    }
    aliases
}
fn catalog(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()
        && !["none", "n/a", "not on label"].contains(&value.to_ascii_lowercase().as_str()))
    .then(|| value.to_owned())
}
impl Lookup {
    pub(super) fn from_env(cache: &Path) -> Result<Option<Self>> {
        let token = ["VCW_DISCOGS_TOKEN", "DISCOGS_TOKEN", "DISCOGS_API_TOKEN"]
            .iter()
            .find_map(|name| std::env::var(name).ok().and_then(Token::new));
        let Some(token) = token else {
            return Ok(None);
        };
        let transport = Arc::new(Http::new()?);
        let cancel = Cancel::new();
        let client = Client::new(ProviderId::Discogs, transport.clone())
            .with_clock(Arc::new(SignalClock {
                clock: SystemClock::new(),
                cancel: cancel.clone(),
            }))
            .with_cache(Arc::new(vcw_metadata::cache::Disk::new(cache)))
            .with_user_agent(format!(
                "bcfunl/{} (private collection metadata)",
                env!("CARGO_PKG_VERSION")
            ));
        Ok(Some(Self {
            provider: Discogs::new(transport)
                .with_client(client)
                .with_genres(vcw_metadata::Genres::empty())
                .with_token(Some(token)),
            cancel,
        }))
    }
    pub(super) fn find(&self, release: &Release, tracks: &[Track]) -> Result<Option<Match>> {
        shutdown::check()?;
        let artist = common(tracks, "ALBUMARTIST")?;
        let artist = if artist.is_empty() {
            &release.artist
        } else {
            &artist
        };
        let artist = super::credits::artist_names(artist)
            .into_iter()
            .next()
            .unwrap_or_default();
        let album = common(tracks, "ALBUM")?;
        let album = if album.is_empty() {
            &release.album
        } else {
            &album
        };
        let album = search_title(&crate::sanitizing::title(album, &artist).0);
        let iso = common(tracks, "RELEASECOUNTRY")?;
        let iso = country(if iso.is_empty() {
            &release.country
        } else {
            &iso
        });
        if artist.is_empty() || album.is_empty() {
            eprintln!("  Discogs unresolved: artist and title are required");
            return Ok(None);
        }
        if iso.is_empty() {
            eprintln!(
                "  Discogs [{artist} - {album}]: source country unknown; only Worldwide matches qualify"
            );
        }
        let aliases = artist_aliases(&artist);
        let mut queries: Vec<_> = aliases
            .iter()
            .map(|alias| Query::new().vinyl_only(false).artist(alias).album(&album))
            .collect();
        // Broaden retrieval only; artist/title/country validation remains mandatory.
        queries.push(Query::new().vinyl_only(false).album(&album));
        let mut ids = std::collections::HashSet::new();
        for query in queries {
            let candidates = self.provider.search(&query, &self.cancel)?;
            eprintln!(
                "  Discogs [{artist} - {album}]: search artist={} returned {} candidates",
                query.artist.as_deref().unwrap_or("<title-only fallback>"),
                candidates.len()
            );
            for candidate in candidates {
                shutdown::check()?;
                let reason = if !aliases
                    .iter()
                    .any(|alias| same_artist(&candidate.artist, alias))
                {
                    Some(format!(
                        "artist {:?} is not a confirmed alias",
                        candidate.artist
                    ))
                } else if !same(&candidate.album, &album) {
                    Some(format!("title {:?} differs", candidate.album))
                } else if !candidate.country.trim().is_empty()
                    && !country_matches(&candidate.country, &iso)
                {
                    Some(format!(
                        "country {:?} differs from {iso}",
                        candidate.country
                    ))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    eprintln!(
                        "  Discogs [{artist} - {album}]: reject {}: {reason}",
                        candidate.id
                    );
                    continue;
                }
                if !ids.insert(candidate.id.clone()) {
                    continue;
                }
                let detail = self.provider.fetch(&candidate.id, &self.cancel)?;
                if aliases
                    .iter()
                    .any(|alias| same_artist(&detail.album_artist, alias))
                    && same(&detail.album, &album)
                    && country_matches(&detail.country, &iso)
                {
                    eprintln!(
                        "  Discogs [{artist} - {album}]: selected {} (catalog {})",
                        detail.id, detail.catalog
                    );
                    return Ok(Some(selected(&detail, &artist, &album, &iso)));
                } else {
                    eprintln!(
                        "  Discogs [{artist} - {album}]: reject detail {}: artist={:?}, title={:?}, country={:?} (expected {iso})",
                        detail.id, detail.album_artist, detail.album, detail.country
                    );
                }
            }
        }
        Ok(None)
    }
}
fn selected(detail: &vcw_metadata::Release, artist: &str, album: &str, iso: &str) -> Match {
    let catalog = catalog(&detail.catalog);
    let genres = crate::genres::normalize(&detail.genres, &[artist.to_owned()]);
    let ids = vec![detail.id.clone()];
    Match {
        catalog: catalog.clone(),
        genres: genres.clone(),
        ids: ids.clone(),
        evidence: json!({"provider":"discogs","kind":"release-match","release_ids":ids,
            "artist":artist,"album":album,"country_iso":iso,"catalog":catalog,"genres":genres,
            "selection_policy":"first-eligible-in-provider-order","genre_policy":"selected-release",
            "matched_artists":[detail.album_artist],"matched_countries":[detail.country],
            "country_policy":"listed-country-or-worldwide"}),
    }
}

pub(super) fn apply(release: &Release, tracks: &mut [Track], found: &Match) -> Release {
    let mut release = release.clone();
    if catalog(&release.catalog).is_none()
        && let Some(cat) = &found.catalog
    {
        release.catalog = cat.clone();
    }
    // A confirmed release collection replaces, rather than merges, Bandcamp tags.
    release.genre = found.genres.join(";");
    for track in tracks {
        if !found.genres.is_empty() {
            track.tags.insert("GENRE".into(), found.genres.clone());
        } else {
            track.tags.remove("GENRE");
        }
        if catalog(&value(&track.tags, "CATALOGNUMBER")).is_none()
            && let Some(cat) = &found.catalog
        {
            track.tags.insert("CATALOGNUMBER".into(), vec![cat.clone()]);
        }
        track.tags.insert("DISCOGSID".into(), found.ids.clone());
        track.tags.insert(
            "BCFUNL_GENRE_SOURCE".into(),
            vec![found.evidence.to_string()],
        );
    }
    release
}

#[cfg(test)]
pub(super) fn fixture() -> Lookup {
    tests::lookup(vec![tests::detail(
        1,
        "DG001",
        json!(["EBM - electronic body music"]),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::Tags;
    use std::sync::Mutex;
    #[derive(Debug)]
    struct Fixture {
        details: Vec<Value>,
        calls: Mutex<Vec<String>>,
    }
    impl Transport for Fixture {
        fn name(&self) -> &'static str {
            "fixture"
        }
        fn send(&self, request: &Request) -> std::result::Result<Response, TransportError> {
            self.calls.lock().unwrap().push(request.url.clone());
            let body = if request.url.contains("/database/search?") {
                assert!(!request.url.to_ascii_lowercase().contains("remaster"));
                json!({"results":self.details.iter().map(|detail| json!({"id":detail["id"],"title":"Romy - Proof","country":detail["country"]})).collect::<Vec<_>>()})
            } else {
                self.details
                    .iter()
                    .find(|detail| {
                        request
                            .url
                            .ends_with(&format!("/{}", detail["id"].as_u64().unwrap()))
                    })
                    .cloned()
                    .unwrap()
            };
            Ok(Response::ok(serde_json::to_vec(&body).unwrap()))
        }
    }
    pub(super) fn detail(id: u64, cat: &str, styles: Value) -> Value {
        json!({"id":id,"title":"Proof","artists":[{"name":"Romy"}],"country":"UK","labels":[{"catno":cat}],"genres":["Electronic"],"styles":styles})
    }
    pub(super) fn lookup(details: Vec<Value>) -> Lookup {
        let transport = Arc::new(Fixture {
            details,
            calls: Mutex::new(Vec::new()),
        });
        let client = Client::new(ProviderId::Discogs, transport.clone())
            .with_clock(Arc::new(vcw_metadata::policy::TestClock::new()));
        Lookup {
            provider: Discogs::new(transport)
                .with_client(client)
                .with_genres(vcw_metadata::Genres::empty())
                .with_token(Token::new("fixture-token")),
            cancel: Cancel::new(),
        }
    }
    #[test]
    fn edition_notes_are_search_only_and_require_a_year() {
        let mclusky = "My Pain And Sadness Is More Sad And Painful Than Yours";
        assert_eq!(search_title(&format!("{mclusky} (2026 Remaster)")), mclusky);
        for suffix in [
            " (2026 Remaster)",
            " [Remastered 2026]",
            " - 2026 Reissue",
            " (2026 RM/RE)",
            " (RE / RM 2026)",
            " - Remastered in 2026",
            " [2026 Digitally Remastered Edition]",
            " 2026 RM",
            " (2026 Re-issued)",
        ] {
            assert_eq!(search_title(&format!("Proof{suffix}")), "Proof", "{suffix}");
            assert!(same(&format!("Proof{suffix}"), "Proof"));
        }
        for title in [
            "1984",
            "Proof (Remaster)",
            "Proof (2026 Remix)",
            "Proof (Live 2026)",
            "Proof (2026 Bonus Tracks)",
            "Remastered 2026",
            "Proof (2026 Remaster by Someone)",
            "Proof (Part 2026)",
        ] {
            assert_eq!(search_title(title), title);
        }
        let (release, mut tracks) = source();
        tracks[0]
            .tags
            .insert("ALBUM".into(), vec!["Proof (2026 Remaster)".into()]);
        let original = tracks[0].tags.clone();
        let found = lookup(vec![detail(1, "ABC123", json!(["House"]))])
            .find(&release, &tracks)
            .unwrap()
            .unwrap();
        assert_eq!(found.catalog.as_deref(), Some("ABC123"));
        assert_eq!(tracks[0].tags, original);
    }

    fn source() -> (Release, Vec<Track>) {
        let mut tags = Tags::new();
        for (k, v) in [
            ("ARTIST", "Romy"),
            ("ALBUMARTIST", "Romy"),
            ("ALBUM", "Proof"),
            ("TITLE", "One Track"),
            ("GENRE", "Bandcamp Junk"),
        ] {
            tags.insert(k.into(), vec![v.into()]);
        }
        let release = Release {
            key: "album1".into(),
            artist: "Romy".into(),
            album: "Proof".into(),
            source: "unused.flac".into(),
            genre: "Bandcamp Junk".into(),
            country: "GB".into(),
            catalog: "".into(),
            publish: "".into(),
            url: "".into(),
        };
        (
            release,
            vec![Track {
                path: "unused.flac".into(),
                tags,
                duration: 1.0,
            }],
        )
    }
    #[test]
    fn release_match_requires_all_three_identity_fields_and_replaces_genres() {
        let (source, mut tracks) = source();
        let found = lookup(vec![detail(
            1,
            "CAT72",
            json!(["EBM - electronic body music"]),
        )])
        .find(&source, &tracks)
        .unwrap()
        .unwrap();
        let enriched = apply(&source, &mut tracks, &found);
        assert_eq!(enriched.catalog, "CAT72");
        assert_eq!(tracks[0].tags["GENRE"], ["Electronic", "EBM"]);
        assert!(!tracks[0].tags["GENRE"].contains(&"Bandcamp Junk".into()));
        assert_eq!(tracks[0].tags["DISCOGSID"], ["1"]);
        assert_eq!(found.evidence["country_iso"], "GB");
        for (field, bad) in [
            ("country", json!("US")),
            ("title", json!("Other Title")),
            ("artists", json!([{"name":"Other Artist"}])),
        ] {
            let mut wrong = detail(2, "CAT72", json!(["EBM"]));
            wrong[field] = bad;
            assert!(
                lookup(vec![wrong])
                    .find(&source, &tracks)
                    .unwrap()
                    .is_none(),
                "{field}"
            );
        }
        let mut unknown = source.clone();
        unknown.country.clear();
        assert!(lookup(vec![]).find(&unknown, &tracks).unwrap().is_none());
    }
    #[test]
    fn first_eligible_release_supplies_all_metadata_and_only_one_id() {
        let (source, tracks) = source();
        let variants = ["DG011", "DG 011", "DG-011", "DG - 011"];
        for (first, second) in variants.iter().flat_map(|first| {
            variants
                .iter()
                .filter(move |second| first != *second)
                .map(move |second| (*first, *second))
        }) {
            let provider = lookup(vec![
                detail(1, first, json!(["EBM"])),
                detail(2, second, json!(["Ambient"])),
            ]);
            let found = provider.find(&source, &tracks).unwrap().unwrap();
            assert_eq!(found.catalog.as_deref(), Some(first));
            assert_eq!(found.ids, ["1"]);
            assert_eq!(found.genres, ["Electronic", "EBM"]);
            let mut release = source.clone();
            let mut tagged = tracks.clone();
            let enriched = apply(&release, &mut tagged, &found);
            assert_eq!(enriched.catalog, first);
            assert_eq!(tagged[0].tags["CATALOGNUMBER"], [first]);
            assert_eq!(tagged[0].tags["DISCOGSID"], ["1"]);
            release.catalog = "existing".into();
            assert_eq!(apply(&release, &mut tagged, &found).catalog, "existing");
            assert_eq!(
                found.evidence["selection_policy"],
                "first-eligible-in-provider-order"
            );
        }
    }

    #[test]
    fn confirmed_catalog_does_not_replace_existing_source_catalog() {
        let (mut source, mut tracks) = source();
        source.catalog = "OWN1".into();
        tracks[0]
            .tags
            .insert("CATALOGNUMBER".into(), vec!["OWN1".into()]);
        let found = lookup(vec![detail(1, "OTHER", json!(["EBM"]))])
            .find(&source, &tracks)
            .unwrap()
            .unwrap();
        let enriched = apply(&source, &mut tracks, &found);
        assert_eq!(enriched.catalog, "OWN1");
        assert_eq!(tracks[0].tags["CATALOGNUMBER"], ["OWN1"]);
    }
    #[derive(Debug)]
    struct Aliased {
        title_only: bool,
        unrelated: bool,
        starred: bool,
        searches: std::sync::atomic::AtomicUsize,
    }
    impl Transport for Aliased {
        fn name(&self) -> &'static str {
            "alias-fixture"
        }
        fn send(&self, request: &Request) -> std::result::Result<Response, TransportError> {
            let body = if request.url.contains("/database/search?") {
                self.searches
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if request.url.contains("artist=Osees")
                    || (self.title_only && request.url.contains("artist="))
                {
                    json!({"results":[]})
                } else {
                    let artist = if self.starred {
                        "oSeEs*"
                    } else if self.unrelated {
                        "Unrelated Band"
                    } else {
                        "Thee Oh Sees"
                    };
                    let region = if self.starred { "Worldwide" } else { "US" };
                    json!({"results":[
                        {"id":1,"title":format!("{artist} - Off Course"),"country":region,"catno":"DG011"},
                        {"id":2,"title":format!("{artist} - Off Course"),"country":"UK","catno":"DG011"}
                    ]})
                }
            } else {
                assert!(
                    request.url.ends_with("/1"),
                    "Wrong-country release must not be fetched"
                );
                let artist = if self.starred {
                    "OSEES*"
                } else {
                    "Thee Oh Sees"
                };
                let region = if self.starred { "Worldwide" } else { "US" };
                json!({"id":1,"title":"Off Course","artists":[{"name":artist}],"country":region,"labels":[{"catno":"DG011"}],"genres":["Rock"],"styles":[]})
            };
            Ok(Response::ok(serde_json::to_vec(&body).unwrap()))
        }
    }
    #[test]
    fn osees_alias_search_and_title_fallback_keep_identity_checks() {
        for (title_only, unrelated, starred) in [
            (false, false, false),
            (true, false, false),
            (true, true, false),
            (false, false, true),
        ] {
            let transport = Arc::new(Aliased {
                title_only,
                unrelated,
                starred,
                searches: Default::default(),
            });
            let client = Client::new(ProviderId::Discogs, transport.clone())
                .with_clock(Arc::new(vcw_metadata::policy::TestClock::new()));
            let lookup = Lookup {
                provider: Discogs::new(transport.clone())
                    .with_client(client)
                    .with_token(Token::new("fixture-token")),
                cancel: Cancel::new(),
            };
            let (mut source, mut tracks) = source();
            source.artist = "Osees".into();
            source.album = "OFF COURSE".into();
            source.country = "US".into();
            tracks[0]
                .tags
                .insert("ALBUMARTIST".into(), vec!["Osees".into()]);
            tracks[0]
                .tags
                .insert("ALBUM".into(), vec!["OFF COURSE".into()]);
            let result = lookup.find(&source, &tracks).unwrap();
            if unrelated {
                assert!(result.is_none());
            } else {
                let matched = result.unwrap();
                assert_eq!(matched.catalog.as_deref(), Some("DG011"));
                assert_eq!(matched.ids, ["1"]);
            }
            assert!(
                transport
                    .searches
                    .load(std::sync::atomic::Ordering::Relaxed)
                    > 1
            );
        }
        assert!(
            !artist_aliases("Nick Cave & The Bad Seeds")
                .iter()
                .any(|name| name == "Nick Cave")
        );
    }
    #[test]
    fn worldwide_matches_any_country_without_changing_source_country() {
        for iso in ["US", "GB", "DE", ""] {
            let (mut source, mut tracks) = source();
            source.country = iso.into();
            let mut release = detail(1, "DG011", json!(["Garage Rock"]));
            release["country"] = json!("Worldwide");
            let found = lookup(vec![release])
                .find(&source, &tracks)
                .unwrap()
                .unwrap();
            assert_eq!(found.catalog.as_deref(), Some("DG011"));
            let enriched = apply(&source, &mut tracks, &found);
            assert_eq!(enriched.country, iso);
            assert_eq!(found.evidence["matched_countries"], json!(["Worldwide"]));
        }
        assert!(!country_matches("UK", "US"));
        assert!(!country_matches("Europe", "GB"));
        assert!(country_matches("Europe", "EU"));
        assert!(!country_matches("", "US"));
        assert!(!country_matches("", ""));
    }
    #[test]
    fn discogs_artist_asterisk_is_ignored_only_at_the_end_and_case_insensitively() {
        assert!(same_artist("Osees*", "OSEES"));
        assert!(same_artist("  OSEES*  ", "osees"));
        assert!(same_artist("RØDEL*", "rødel"));
        assert!(!same_artist("O*sees", "Osees"));
        assert!(!same_artist("*", ""));
        assert!(!same("Title*", "Title"));
    }
    #[test]
    fn composite_country_matches_listed_members_only() {
        for (listed, iso) in [
            ("UK & US", "US"),
            ("UK&US", "US"),
            ("UK & US", "GB"),
            ("Europe & UK", "EU"),
            ("Europe & UK", "GB"),
            ("US, Canada", "CA"),
            ("UK/US", "US"),
        ] {
            assert!(country_matches(listed, iso), "{listed}: {iso}");
        }
        assert!(!country_matches("UK & US", "DE"));
        assert!(!country_matches("UK & US", ""));
        let (source, tracks) = source();
        let mut release = detail(1, "DG011", json!(["EBM"]));
        release["country"] = json!("UK & US");
        let found = lookup(vec![release])
            .find(&source, &tracks)
            .unwrap()
            .unwrap();
        assert_eq!(found.catalog.as_deref(), Some("DG011"));
    }
    #[test]
    #[ignore = "manual read-only replay of existing Discogs cache"]
    fn cached_off_course_response_applies_catalog_and_genres() {
        #[derive(Debug)]
        struct Cached(vcw_metadata::cache::Disk);
        impl Transport for Cached {
            fn name(&self) -> &'static str {
                "cached-fixture"
            }
            fn send(&self, request: &Request) -> std::result::Result<Response, TransportError> {
                let key = vcw_metadata::cache::key(ProviderId::Discogs, &request.url);
                self.0
                    .load(&key)
                    .ok()
                    .flatten()
                    .map(Response::ok)
                    .ok_or_else(|| TransportError::Protocol("No unexpired cached fixture".into()))
            }
        }
        let path = std::env::var_os("BCFUNL_TEST_DISCOGS_CACHE").expect("cache path required");
        let provider = Discogs::new(Arc::new(Cached(vcw_metadata::cache::Disk::new(path))))
            .with_genres(vcw_metadata::Genres::empty())
            .with_token(Token::new("fixture-token"));
        let detail = provider.fetch("37546071", &Cancel::new()).unwrap();
        assert!(
            artist_aliases("Osees")
                .iter()
                .any(|alias| same_artist(&detail.album_artist, alias))
        );
        assert!(same(&detail.album, "OFF COURSE"));
        assert!(country_matches(&detail.country, "US"));
        let found = selected(&detail, "Osees", "OFF COURSE", "US");
        let (mut source, mut tracks) = source();
        source.country = "US".into();
        let enriched = apply(&source, &mut tracks, &found);
        assert_eq!(enriched.catalog, "DG 011");
        assert_eq!(tracks[0].tags["CATALOGNUMBER"], ["DG 011"]);
        assert_eq!(tracks[0].tags["DISCOGSID"], ["37546071"]);
        assert_eq!(tracks[0].tags["GENRE"], ["Rock", "Krautrock"]);
        assert_eq!(enriched.country, "US");
        eprintln!(
            "Cached actual Off Course response: catalog DG 011, genres Rock;Krautrock, one Discogs ID37546071 applied; no live requests or music writes."
        );
    }
}
