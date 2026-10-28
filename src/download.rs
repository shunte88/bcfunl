use crate::{Args, Format, Release, flagged, metadata_date, net::Network};
use anyhow::{Context, Result, ensure};
use chrono::{NaiveDate, Utc};
use scraper::{Html, Selector};
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct InvalidPayload(&'static str);
impl std::fmt::Display for InvalidPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for InvalidPayload {}

#[derive(Debug)]
struct VerificationRequired;
impl std::fmt::Display for VerificationRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Bandcamp requires browser verification")
    }
}
impl std::error::Error for VerificationRequired {}

pub struct Downloaded {
    pub path: PathBuf,
    pub format: String,
    pub publish: Option<NaiveDate>,
}

fn number(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
        .max(0.0)
}
fn quality(value: &Value) -> (u32, u32) {
    let bits = ["bit_depth", "bits_per_sample"]
        .iter()
        .map(|k| number(&value[*k]) as u32)
        .max()
        .unwrap_or(0);
    let rate = ["sample_rate", "sample_rate_hz"]
        .iter()
        .map(|k| {
            let n = number(&value[*k]);
            (if n > 0.0 && n < 1000.0 { n * 1000.0 } else { n }) as u32
        })
        .max()
        .unwrap_or(0);
    (bits, rate)
}
const FORMAT_EXTENSIONS: [(&str, &str); 8] = [
    ("flac", "flac"),
    ("wav", "wav"),
    ("aiff-lossless", "aiff"),
    ("alac", "m4a"),
    ("aac-hi", "m4a"),
    ("vorbis", "ogg"),
    ("mp3-320", "mp3"),
    ("mp3-v0", "mp3"),
];
fn choose(downloads: &Value, preferred: Format) -> Result<(String, String)> {
    let formats = downloads
        .as_object()
        .context("Release has no downloadable formats")?;
    let preferred = match preferred {
        Format::Flac => "flac",
        Format::Wav => "wav",
        Format::Alac => "alac",
        Format::AiffLossless => "aiff-lossless",
        _ => "",
    };
    let mut candidates = Vec::new();
    for (index, (name, _)) in FORMAT_EXTENSIONS.iter().enumerate() {
        let Some(value) = formats.get(*name) else {
            continue;
        };
        let Some(url) = value["url"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let lossless = index < 4;
        let (bits, rate) = if lossless { quality(value) } else { (0, 0) };
        // Lossless resolution wins; requested lossless format breaks ties before the fallback order.
        let rank = (
            lossless,
            bits,
            rate,
            lossless && *name == preferred,
            FORMAT_EXTENSIONS.len() - index,
        );
        candidates.push((rank, name.to_string(), url.to_owned()));
    }
    candidates.sort_by_key(|a| std::cmp::Reverse(a.0));
    candidates
        .into_iter()
        .next()
        .map(|(_, name, url)| (name, url))
        .context("No supported audio download available")
}
fn safe_name(value: &str) -> String {
    let mut cleaned = String::new();
    for c in value.chars().map(|c| {
        if c.is_control() || "<>:\"/\\|?*".contains(c) {
            '_'
        } else {
            c
        }
    }) {
        if cleaned.len() + c.len_utf8() > 90 {
            break;
        }
        cleaned.push(c);
    }
    let cleaned = cleaned.trim_matches([' ', '.']);
    if cleaned.is_empty() {
        "untitled".into()
    } else {
        cleaned.into()
    }
}

fn release_metadata(html: &str) -> Result<(Vec<NaiveDate>, bool, Option<bool>)> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse("[data-tralbum]").unwrap();
    if let Some(blob) = doc
        .select(&selector)
        .find_map(|e| e.value().attr("data-tralbum"))
    {
        let data: Value = serde_json::from_str(blob).context("Invalid release metadata")?;
        let mut dates = crate::collection::release_dates(&data);
        dates.extend(crate::collection::release_dates(&data["current"]));
        let preorder = flagged(&data)
            || flagged(&data["current"])
            || data["album_is_preorder"].as_bool().unwrap_or(false);
        let single = match (
            crate::collection::single_track(&data),
            crate::collection::single_track(&data["current"]),
        ) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), _) | (_, Some(true)) => Some(true),
            _ => None,
        };
        return Ok((dates, preorder, single));
    }
    Ok((Vec::new(), false, None))
}

pub(crate) fn save(
    mut reader: impl Read,
    expected: Option<u64>,
    root: &Path,
    basename: &str,
    format: &str,
    force: bool,
) -> Result<PathBuf> {
    crate::shutdown::check()?;
    fs::create_dir_all(root).context("Create release directory")?;
    let mut head = [0u8; 12];
    let mut used = 0;
    while used < head.len() {
        crate::shutdown::check()?;
        let n = reader
            .read(&mut head[used..])
            .context("Read audio header")?;
        if n == 0 {
            break;
        }
        used += n;
    }
    ensure!(used > 0, InvalidPayload("Download was empty"));
    let zip = head.starts_with(b"PK\x03\x04");
    let flac = head.starts_with(b"fLaC");
    let wav = (head.starts_with(b"RIFF") || head.starts_with(b"RF64")) && &head[8..12] == b"WAVE";
    let mp3 = head.starts_with(b"ID3") || (head[0] == 0xff && head[1] & 0xe0 == 0xe0);
    let aiff = head.starts_with(b"FORM") && (&head[8..12] == b"AIFF" || &head[8..12] == b"AIFC");
    let m4a = &head[4..8] == b"ftyp";
    let ogg = head.starts_with(b"OggS");
    let extension = FORMAT_EXTENSIONS
        .iter()
        .find(|(name, _)| *name == format)
        .map(|(_, extension)| *extension)
        .context("Unsupported audio format")?;
    let matches = match format {
        "flac" => flac,
        "wav" => wav,
        "alac" | "aac-hi" => m4a,
        "aiff-lossless" => aiff,
        "vorbis" => ogg,
        "mp3-320" | "mp3-v0" => mp3,
        _ => false,
    };
    // shortcut: header and length checks only; add full decoding if archive/audio corruption occurs.
    ensure!(
        zip || matches,
        InvalidPayload("Unexpected audio/archive payload (possibly an HTML error page)")
    );
    let extension = if zip { "zip" } else { extension };
    let target = root.join(format!("{basename} [{format}].{extension}"));
    ensure!(
        force || !target.exists(),
        "Target exists without a recorded completion; use --force to replace it"
    );
    let part = target.with_extension(format!("{extension}.part"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&part)
        .context("Create partial file (remove a stale .part file before retrying)")?;
    let result = (|| {
        file.write_all(&head[..used])
            .context("Write audio header")?;
        let mut bytes = used as u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            crate::shutdown::check()?;
            let n = reader.read(&mut buffer).context("Read audio download")?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n])
                .context("Write audio download")?;
            bytes += n as u64;
        }
        crate::shutdown::check()?;
        ensure!(
            expected.is_none_or(|size| size == bytes),
            "Incomplete download: expected {} bytes, received {bytes}",
            expected.unwrap_or(0)
        );
        file.sync_all().context("Sync audio file")?;
        drop(file);
        if force {
            fs::rename(&part, &target).context("Replace completed file")?;
        } else {
            fs::hard_link(&part, &target).context("Publish file without overwriting")?;
            fs::remove_file(&part)?;
        }
        File::open(root)?
            .sync_all()
            .context("Sync release directory")?;
        Ok(target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

fn stat_url(url: &str) -> Result<Option<reqwest::Url>> {
    let mut url = reqwest::Url::parse(url).context("Invalid download URL")?;
    ensure!(
        crate::net::allowed(&url, true),
        "Unexpected audio URL origin"
    );
    let broker = url
        .host_str()
        .is_some_and(|h| h == "bandcamp.com" || h.ends_with(".bandcamp.com"));
    if !broker || !matches!(url.path(), "/download/album" | "/download/track") {
        return Ok(None);
    }
    url.set_path(&url.path().replacen("/download/", "/statdownload/", 1));
    url.query_pairs_mut()
        .append_pair(".rand", &rand::random::<u64>().to_string())
        .append_pair(".vrs", "1");
    Ok(Some(url))
}
fn ready_url(text: &str, base: &reqwest::Url) -> Result<Option<String>> {
    // Some Bandcamp status responses are JSONP; parse the JSON object without executing JavaScript.
    let text = text
        .split_once("Downloads.statResult(")
        .map(|(_, json)| json)
        .unwrap_or(text);
    let start = text
        .find('{')
        .context("Bandcamp status did not contain JSON")?;
    let data = serde_json::Deserializer::from_str(&text[start..])
        .into_iter::<Value>()
        .next()
        .context("Bandcamp status did not contain JSON")?
        .context("Invalid Bandcamp status JSON")?;
    let challenge = data["errortype"]
        .as_str()
        .unwrap_or("")
        .to_ascii_lowercase();
    ensure!(
        !challenge.contains("captcha") && !challenge.contains("challenge"),
        VerificationRequired
    );
    if let Some(next) = data["download_url"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| data["retry_url"].as_str().filter(|s| !s.is_empty()))
    {
        let next = base.join(next).context("Invalid resolved audio URL")?;
        ensure!(
            crate::net::allowed(&next, true),
            "Unexpected resolved audio URL origin"
        );
        return Ok(Some(next.to_string()));
    }
    ensure!(
        data["result"] == "pending" || data["errortype"] == "DownloadNotReadyError",
        "Bandcamp did not provide a downloadable audio URL"
    );
    Ok(None)
}
fn resolve_download(net: &Network, url: String) -> Result<String> {
    let Some(status) = stat_url(&url)? else {
        return Ok(url);
    };
    for _ in 0..12 {
        let text = net
            .get(status.as_str(), true)?
            .text()
            .context("Read Bandcamp download status")?;
        if let Some(url) = ready_url(&text, &status)? {
            return Ok(url);
        }
    }
    anyhow::bail!("Bandcamp archive is still being prepared; retry this release later")
}
pub fn prepare(
    net: &Network,
    args: &Args,
    release: &Release,
    db: &crate::db::Database,
) -> Result<Prepared> {
    let data = net.page(&release.url).context("Read download page")?;

    let items = data["download_items"]
        .as_array()
        .context("Missing download_items")?;
    let captured_title = db
        .query(
            "SELECT release_title FROM releases WHERE user=? AND item_key=?",
            &[&args.user, &release.key],
        )?
        .into_iter()
        .next()
        .and_then(|row| row.into_iter().next().flatten())
        .unwrap_or_else(|| release.title.clone());
    let item = if items.len() == 1 {
        &items[0]
    } else {
        items
            .iter()
            .find(|item| {
                item["title"]
                    .as_str()
                    .is_some_and(|title| crate::initcap(title) == crate::initcap(&captured_title))
            })
            .context("Cannot identify release on download page")?
    };

    let mut publish = metadata_date(
        item,
        &["release_date", "album_release_date", "publish_date"],
    )
    .or_else(|| metadata_date(&data, &["release_date", "album_release_date"]))
    .or(release.publish);
    let mut dates = crate::collection::release_dates(item);
    dates.extend(crate::collection::release_dates(&data));
    dates.extend(release.publish);
    let mut preorder = flagged(&data) || flagged(item);
    let single_evidence = [
        crate::collection::single_track(item),
        crate::collection::single_track(&data),
    ];
    let mut single = !single_evidence.contains(&Some(false))
        && (single_evidence.contains(&Some(true)) || release.key.starts_with("track"));
    // Private purchases remain downloadable even when their public album page returns 404.
    if !release.private
        && let Some(url) = &release.release_url
    {
        let html = net
            .release_page(url, release.release_custom_host.as_deref())
            .context("Fetch release metadata page")?
            .text()
            .context("Read release metadata page")?;
        let (page_dates, page_preorder, page_single) = release_metadata(&html)?;
        preorder |= page_preorder;
        if page_single == Some(false) {
            single = false;
        } else if page_single == Some(true) && !single_evidence.contains(&Some(false)) {
            single = true;
        }
        publish = publish.or(page_dates.first().copied());
        dates.extend(page_dates);
        crate::metadata::store(db, &args.user, &release.key, &html)?;
    }
    ensure!(
        publish.is_none_or(|d| d <= Utc::now().date_naive()),
        "Release publication date is in the future"
    );
    let stale =
        preorder && crate::collection::released_single(single, &dates, Utc::now().date_naive());
    ensure!(!preorder || stale, "Release is a preorder/prerelease");
    let (format, url) = choose(&item["downloads"], args.format)?;
    if stale {
        let cleaned = crate::sanitizing::remove_prerelease(&release.title);
        ensure!(
            !cleaned.is_empty(),
            "Missing title after removing stale prerelease marker"
        );
        db.query(
            "UPDATE releases SET sanitized_title=? WHERE user=? AND item_key=?",
            &[&cleaned, &args.user, &release.key],
        )?;
        eprintln!(
            "  Ignoring stale preorder flag for a confirmed single-track release dated before today"
        );
    }

    let root = args
        .output
        .join(safe_name(&args.user))
        .join(safe_name(&release.artist));
    let output_title = if stale {
        crate::sanitizing::remove_prerelease(&release.title)
    } else {
        release.title.clone()
    };
    let basename = format!("{} - {}", safe_name(&output_title), safe_name(&release.key));
    Ok(Prepared {
        title: item["title"].as_str().unwrap_or(&release.title).into(),
        artist: item["artist"].as_str().unwrap_or(&release.artist).into(),
        url,
        format,
        publish,
        released_single: crate::collection::released_single(
            single,
            &dates,
            Utc::now().date_naive(),
        ),
        root,
        basename,
    })
}

pub struct Prepared {
    pub url: String,
    title: String,
    artist: String,
    format: String,
    publish: Option<NaiveDate>,
    released_single: bool,
    root: PathBuf,
    basename: String,
}
impl Prepared {
    #[cfg(test)]
    pub(crate) fn fixture(root: PathBuf) -> Self {
        Self {
            url: "https://bandcamp.com/download?fixture".into(),
            title: "Title".into(),
            artist: "Artist".into(),
            format: "flac".into(),
            publish: None,
            released_single: false,
            root,
            basename: "release".into(),
        }
    }

    pub fn save(
        &self,
        reader: impl Read,
        expected: Option<u64>,
        force: bool,
    ) -> Result<Downloaded> {
        let path = save(
            reader,
            expected,
            &self.root,
            &self.basename,
            &self.format,
            force,
        )?;
        Ok(Downloaded {
            path,
            format: self.format.clone(),
            publish: self.publish,
        })
    }
}
const HTML_LIMIT: u64 = 2 * 1024 * 1024;
pub(crate) fn read_html(reader: impl Read) -> Result<String> {
    let mut data = Vec::new();
    reader
        .take(HTML_LIMIT + 1)
        .read_to_end(&mut data)
        .context("Read returned download page")?;
    ensure!(
        data.len() as u64 <= HTML_LIMIT,
        "Returned download page exceeds 2 MiB"
    );
    String::from_utf8(data).context("Invalid payload was not a UTF-8 download page")
}
fn refreshed_link(
    html: &str,
    title: &str,
    artist: &str,
    preferred: Format,
    previous: Option<(bool, NaiveDate)>,
) -> Result<(String, String)> {
    let data = crate::collection::pagedata(html)
        .context("Returned payload has no Bandcamp download links")?;

    let items = data["download_items"]
        .as_array()
        .context("Returned page has no download_items")?;
    let item = items
        .iter()
        .find(|item| {
            item["title"]
                .as_str()
                .is_some_and(|t| t == title || crate::initcap(t) == crate::initcap(title))
                && item["artist"].as_str().is_none_or(|a| {
                    a == artist
                        || crate::sanitizing::artist_display(a)
                            == crate::sanitizing::artist_display(artist)
                })
        })
        .context("Returned download page belongs to a different release")?;
    let evidence = [
        crate::collection::single_track(item),
        crate::collection::single_track(&data),
    ];
    let single = !evidence.contains(&Some(false))
        && (evidence.contains(&Some(true)) || previous.is_some_and(|(single, _)| single));
    let mut dates = crate::collection::release_dates(item);
    dates.extend(crate::collection::release_dates(&data));
    dates.extend(previous.map(|(_, date)| date));
    let today = Utc::now().date_naive();
    ensure!(
        dates.iter().all(|date| *date <= today),
        "Returned release publication date is in the future"
    );
    ensure!(
        !(flagged(&data) || flagged(item))
            || crate::collection::released_single(single, &dates, today),
        "Returned page is a preorder/prerelease"
    );
    let (format, url) = choose(&item["downloads"], preferred)?;
    ensure!(
        crate::net::allowed(&reqwest::Url::parse(&url)?, true),
        "Unexpected refreshed audio URL origin"
    );
    Ok((format, url))
}
impl Prepared {
    pub(crate) fn refresh(&mut self, net: &Network, html: &str, preferred: Format) -> Result<()> {
        let (format, url) = refreshed_link(
            html,
            &self.title,
            &self.artist,
            preferred,
            self.publish.map(|date| (self.released_single, date)),
        )?;
        self.format = format;
        // Status lookup is helpful for stale links, but some purchased URLs work only directly.
        self.url = match resolve_download(net, url.clone()) {
            Ok(resolved) => resolved,
            Err(error) if error.is::<VerificationRequired>() => return Err(error),
            Err(error) => {
                crate::shutdown::check()?;
                eprintln!(
                    "  {} - {}: status lookup failed ({error:#}); retrying the page's direct link.",
                    self.artist, self.title
                );
                url
            }
        };
        eprintln!(
            "  {} - {}: retrying a refreshed {} link.",
            self.artist, self.title, self.format
        );
        Ok(())
    }
}
pub(crate) enum TransferResult {
    Complete(Downloaded),
    Page(String),
}
pub(crate) fn with_recovery(
    mut prepared: Prepared,
    mut transfer: impl FnMut(&Prepared) -> Result<TransferResult>,
    mut refresh: impl FnMut(&mut Prepared, &str) -> Result<()>,
) -> Result<Downloaded> {
    for attempt in 0..3 {
        crate::shutdown::check()?;
        match transfer(&prepared)? {
            TransferResult::Complete(done) => return Ok(done),
            TransferResult::Page(html) => {
                ensure!(
                    attempt < 2,
                    "Bandcamp returned download-page HTML after 3 download attempts"
                );
                refresh(&mut prepared, &html)
                    .context("Refresh link from returned download page")?;
            }
        }
    }
    unreachable!()
}
pub fn fetch(
    net: &Network,
    args: &Args,
    release: &Release,
    db: &crate::db::Database,
) -> Result<Downloaded> {
    let prepared = prepare(net, args, release, db)?;
    with_recovery(
        prepared,
        |prepared| {
            let mut response = net
                .get(&prepared.url, true)
                .context("Fetch selected audio format")?;
            let expected = response.content_length();
            // Keep the prefix so successful audio is streamed once, without a preflight download.
            let mut head = [0u8; 12];
            let mut used = 0;
            while used < head.len() {
                let n = response
                    .read(&mut head[used..])
                    .context("Read payload header")?;
                if n == 0 {
                    break;
                }
                used += n;
            }
            let prefix = &head[..used];
            let looks_html = String::from_utf8_lossy(prefix)
                .trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
                .starts_with('<');
            if looks_html {
                return Ok(TransferResult::Page(read_html(
                    std::io::Cursor::new(prefix).chain(response),
                )?));
            }
            prepared
                .save(
                    std::io::Cursor::new(prefix).chain(response),
                    expected,
                    args.force,
                )
                .map(TransferResult::Complete)
        },
        |prepared, html| prepared.refresh(net, html, args.format),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn format_preferences_fallbacks_and_resolution() {
        let values = json!({"flac":{"url":"f"},"wav":{"url":"w"},"mp3-320":{"url":"m"},"mp3-v0":{"url":"v"}});
        assert_eq!(choose(&values, Format::Flac).unwrap().0, "flac");
        assert_eq!(choose(&values, Format::Wav).unwrap().0, "wav");
        assert_eq!(
            choose(&json!({"wav":{"url":"w"}}), Format::Flac).unwrap().0,
            "wav"
        );
        assert_eq!(
            choose(&json!({"flac":{"url":"f"}}), Format::Wav).unwrap().0,
            "flac"
        );
        assert_eq!(
            choose(
                &json!({"mp3-v0":{"url":"v"},"mp3-320":{"url":"m"}}),
                Format::Flac
            )
            .unwrap()
            .0,
            "mp3-320"
        );
        assert_eq!(choose(&json!({"flac":{"url":"f","bit_depth":16,"sample_rate":44100},"wav":{"url":"w","bit_depth":24,"sample_rate":96000}}),Format::Flac).unwrap().0,"wav");
        assert!(choose(&json!({"flac":{"url":null}}), Format::Flac).is_err());
    }
    #[test]
    fn complete_fallback_order_and_lossless_preferences() {
        let mut formats = serde_json::Map::new();
        for (name, _) in FORMAT_EXTENSIONS {
            formats.insert(
                name.into(),
                json!({"url":format!("https://bandcamp.com/{name}")}),
            );
        }
        for (expected, _) in FORMAT_EXTENSIONS {
            assert_eq!(
                choose(&Value::Object(formats.clone()), Format::Flac)
                    .unwrap()
                    .0,
                expected
            );
            formats.remove(expected);
        }
        assert!(choose(&Value::Object(formats), Format::Flac).is_err());
        let lossless = json!({"aiff-lossless":{"url":"a"},"alac":{"url":"b"},"aac-hi":{"url":"c","bit_depth":32,"sample_rate":192000}});
        assert_eq!(choose(&lossless, Format::Flac).unwrap().0, "aiff-lossless");
        assert_eq!(choose(&lossless, Format::Alac).unwrap().0, "alac");
        assert_eq!(choose(&lossless, Format::Mp3).unwrap().0, "aiff-lossless");
        assert_eq!(choose(&json!({"alac":{"url":"a","bit_depth":24,"sample_rate":48000},"aiff-lossless":{"url":"b","bit_depth":24,"sample_rate":96000}}),Format::Alac).unwrap().0,"aiff-lossless");
    }
    #[test]
    fn saves_every_supported_single_track_extension_and_rejects_html() {
        let root = crate::tests::tempdir();
        for (format, extension) in FORMAT_EXTENSIONS {
            let payload: &[u8] = match format {
                "flac" => b"fLaCfixture-audio",
                "wav" => b"RIFF1234WAVEfixture-audio",
                "aiff-lossless" => b"FORM1234AIFFfixture-audio",
                "alac" | "aac-hi" => b"0000ftypM4A fixture-audio",
                "vorbis" => b"OggSfixture-audio",
                _ => b"ID3fixture-audio",
            };
            let path = save(
                payload,
                Some(payload.len() as u64),
                &root,
                format,
                format,
                false,
            )
            .unwrap();
            assert_eq!(path.extension().unwrap(), extension);
            assert_eq!(fs::read(path).unwrap(), payload);
            assert!(
                save(
                    &b"<html>failed</html>"[..],
                    None,
                    &root,
                    "bad",
                    format,
                    false
                )
                .is_err()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn resolves_download_broker_urls_and_parses_status_without_executing_javascript() {
        let status =
            stat_url("https://popplers5.bandcamp.com/download/album?enc=flac&id=42&sig=fixture")
                .unwrap()
                .unwrap();
        assert_eq!(status.path(), "/statdownload/album");
        assert!(
            status
                .query_pairs()
                .any(|(key, value)| key == "enc" && value == "flac")
        );
        assert!(
            stat_url("https://p4.bcbits.com/download/album/fixture")
                .unwrap()
                .is_none()
        );
        assert!(stat_url("https://example.com/download/album").is_err());
        for text in [
            r#"{"result":"ok","download_url":"https://p4.bcbits.com/download/album/fixture"}"#,
            r#"if (window.Downloads) { Downloads.statResult({"result":"err","retry_url":"https://p4.bcbits.com/download/album/fixture"}) };"#,
        ] {
            assert_eq!(
                ready_url(text, &status).unwrap().as_deref(),
                Some("https://p4.bcbits.com/download/album/fixture")
            );
        }
        assert!(
            ready_url(r#"{"result":"pending"}"#, &status)
                .unwrap()
                .is_none()
        );
        assert!(
            ready_url(
                r#"{"result":"err","errortype":"DownloadNotReadyError"}"#,
                &status
            )
            .unwrap()
            .is_none()
        );
        assert!(ready_url(r#"{"retry_url":"https://example.com/private"}"#, &status).is_err());
        assert!(
            ready_url(
                r#"{"errortype":"CaptchaRequired","retry_url":"https://p4.bcbits.com/x"}"#,
                &status
            )
            .is_err()
        );
        assert!(ready_url("<html>not JSON</html>", &status).is_err());
    }
    fn recovery_page(title: &str, artist: &str, url: &str) -> String {
        let data = json!({"download_items":[{"title":title,"artist":artist,"downloads":{"flac":{"url":url}}}]});
        format!(
            "<div id='pagedata' data-blob='{}'></div>",
            data.to_string()
                .replace('&', "&amp;")
                .replace('\'', "&#39;")
        )
    }
    #[test]
    fn recovery_accepts_only_old_confirmed_single_track_preorders() {
        let today = Utc::now().date_naive();
        let past = today.pred_opt().unwrap();
        for (single, date, expected) in [
            (true, past, true),
            (true, today, false),
            (false, past, false),
        ] {
            let data = json!({"is_preorder":true,"download_items":[{"title":"Title","artist":"Artist","downloads":{"flac":{"url":"https://popplers5.bandcamp.com/download/track?fixture"}}}]});
            let html = format!("<div id='pagedata' data-blob='{data}'></div>");
            assert_eq!(
                refreshed_link(&html, "Title", "Artist", Format::Flac, Some((single, date)))
                    .is_ok(),
                expected
            );
        }
        let data = json!({"is_preorder":true,"release_date":today.to_string(),"download_items":[{"type":"track","title":"Title","artist":"Artist","downloads":{"flac":{"url":"https://popplers5.bandcamp.com/download/track?fixture"}}}]});
        let html = format!("<div id='pagedata' data-blob='{data}'></div>");
        assert!(
            refreshed_link(&html, "Title", "Artist", Format::Flac, Some((true, past))).is_err()
        );
        let html = "<script data-tralbum='{&quot;album_release_date&quot;:&quot;2023-01-01&quot;,&quot;is_preorder&quot;:true,&quot;trackinfo&quot;:[{}]}'></script>";
        let (dates, preorder, single) = release_metadata(html).unwrap();
        assert!(preorder);
        assert!(crate::collection::released_single(
            single == Some(true),
            &dates,
            today
        ));
    }

    #[test]
    fn html_link_recovery_validates_identity_and_origin() {
        let html = recovery_page(
            "Title",
            "Artist",
            "https://popplers5.bandcamp.com/download/album?sig=fixture&enc=flac",
        );
        let (format, url) = refreshed_link(&html, "Title", "Artist", Format::Flac, None).unwrap();
        assert_eq!(format, "flac");
        assert!(url.ends_with("sig=fixture&enc=flac"));
        assert!(refreshed_link(&html, "Other Title", "Artist", Format::Flac, None).is_err());
        assert!(refreshed_link(&html, "Title", "Other Artist", Format::Flac, None).is_err());
        let html = recovery_page("Title", "Artist", "https://example.com/private");
        assert!(refreshed_link(&html, "Title", "Artist", Format::Flac, None).is_err());
        assert!(
            refreshed_link(
                "<html>verification</html>",
                "Title",
                "Artist",
                Format::Flac,
                None
            )
            .is_err()
        );
        assert!(read_html(std::io::repeat(b'x').take(HTML_LIMIT + 1)).is_err());
    }
    #[test]
    fn recovery_downloads_html_then_fresh_audio_and_stops_after_three_pages() {
        let root = crate::tests::tempdir();
        let mut attempts = 0;
        let mut refreshes = 0;
        let done = with_recovery(
            Prepared::fixture(root.clone()),
            |prepared| {
                attempts += 1;
                if attempts == 1 {
                    return Ok(TransferResult::Page(recovery_page(
                        "Title",
                        "Artist",
                        "https://p4.bcbits.com/fresh",
                    )));
                }
                assert_eq!(prepared.url, "https://p4.bcbits.com/fresh");
                prepared
                    .save(&b"fLaCfixture-complete"[..], None, false)
                    .map(TransferResult::Complete)
            },
            |prepared, html| {
                refreshes += 1;
                let (format, url) =
                    refreshed_link(html, &prepared.title, &prepared.artist, Format::Flac, None)?;
                prepared.format = format;
                prepared.url = url;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(refreshes, 1);
        assert_eq!(fs::read(done.path).unwrap(), b"fLaCfixture-complete");
        attempts = 0;
        refreshes = 0;
        let result = with_recovery(
            Prepared::fixture(root.clone()),
            |_| {
                attempts += 1;
                Ok(TransferResult::Page("same page".into()))
            },
            |_, _| {
                refreshes += 1;
                Ok(())
            },
        );
        assert!(result.err().unwrap().to_string().contains("after 3"));
        assert_eq!(attempts, 3);
        assert_eq!(refreshes, 2);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_traversal_and_reads_release_metadata() {
        assert!(safe_name(&"音".repeat(90)).len() <= 90);
        assert_eq!(
            quality(&json!({"sample_rate":44.1,"bit_depth":24})),
            (24, 44100)
        );
        assert!(!safe_name("../a/b\\c").contains('/'));
        assert_eq!(safe_name(".."), "untitled");
        let html = "<script data-tralbum='{&quot;album_release_date&quot;:&quot;01 Oct 2025 00:00:00 GMT&quot;,&quot;current&quot;:{&quot;is_preorder&quot;:true}}'></script>";
        let (dates, preorder, single) = release_metadata(html).unwrap();
        assert_eq!(dates[0].to_string(), "2025-10-01");
        assert_eq!(single, None);
        assert!(preorder);
    }
    #[test]
    fn partial_download_never_replaces_existing_file() {
        let root = crate::tests::tempdir();
        let payload = b"fLaC01234567890123456789";
        let target = save(
            &payload[..],
            Some(payload.len() as u64),
            &root,
            "song",
            "flac",
            false,
        )
        .unwrap();
        assert!(save(&payload[..], None, &root, "song", "flac", false).is_err());
        assert!(save(&payload[..], Some(900), &root, "song", "flac", true).is_err());
        assert_eq!(fs::read(&target).unwrap(), payload);
        assert!(
            save(
                &b"<html>failed</html>"[..],
                None,
                &root,
                "error",
                "flac",
                false
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn saves_album_zip_without_unpacking() {
        let root = crate::tests::tempdir();
        let payload = b"PK\x03\x04fake-test-archive";
        let path = save(&payload[..], None, &root, "album", "wav", false).unwrap();
        assert_eq!(path.extension().unwrap(), "zip");
        fs::remove_dir_all(root).unwrap();
    }
}
