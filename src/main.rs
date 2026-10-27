mod aria2;
mod collection;
mod db;
mod download;
mod metadata;
mod net;
mod processing;
mod resources;
mod sanitizing;
mod shutdown;

use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use clap::{Parser, ValueEnum};
use collection::{flagged, item_key};
use db::Database;
use net::Network;
use serde_json::{Value, json};
use std::{collections::HashSet, fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    Flac,
    Wav,
    Alac,
    AiffLossless,
    AacHi,
    Vorbis,
    #[value(name = "mp3-320")]
    Mp3320,
    #[value(name = "mp3-v0")]
    Mp3V0,
    Mp3,
}

#[derive(Debug, Parser)]
#[command(
    version = concat!("v", env!("CARGO_PKG_VERSION"), " (built ", env!("BCFUNL_BUILD_DATE"), ")"),
    about = "Download and process a Bandcamp collection with SQLite history"
)]
struct Args {
    #[arg(long, default_value = "stuarthunter")]
    user: String,
    /// Case-insensitive artist substring, in collection search or offline processing
    #[arg(long)]
    artist: Option<String>,
    /// Inclusive UTC purchase date, YYYY-MM-DD
    #[arg(long)]
    fromdt: Option<NaiveDate>,
    /// Inclusive UTC purchase date, YYYY-MM-DD
    #[arg(long)]
    todt: Option<NaiveDate>,
    /// Maximum eligible releases to attempt; omit to process the collection
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    count: Option<u32>,
    /// Run 2-8 downloads or release-processing jobs at once; omit for sequential work
    #[arg(long, value_parser = clap::value_parser!(u8).range(2..=8))]
    parallel: Option<u8>,
    #[arg(long, default_value = "downloads")]
    output: PathBuf,
    #[arg(long, value_enum, ignore_case = true, default_value = "flac")]
    format: Format,
    /// Retry completed work; processing never replaces existing server releases
    #[arg(long)]
    force: bool,
    /// Print a status table for releases attempted since this process started
    #[arg(long)]
    summary: bool,
    /// Tag and publish completed downloads without network lookups
    #[arg(long)]
    process: bool,
    /// Acquire/import current Discogs artists and masters snapshots into resources/library.sqlite3
    #[arg(long, conflicts_with_all = ["process", "artist", "fromdt", "todt", "count", "parallel", "serverdir", "template"])]
    resources: bool,
    /// Import an existing Discogs artists XML or XML.gz instead of downloading
    #[arg(long, requires = "resources")]
    resource_file: Option<PathBuf>,
    /// Import an existing Discogs masters XML or XML.gz instead of downloading
    #[arg(long, requires = "resources")]
    master_file: Option<PathBuf>,
    /// Destination music library for --process
    #[arg(long, requires = "process", required_if_eq("process", "true"))]
    serverdir: Option<PathBuf>,
    /// Discogs research SQLite library for missing-genre fallback (default: <output>/resources/library.sqlite3)
    #[arg(long, requires = "process")]
    library: Option<PathBuf>,
    /// Relative output naming rule; the audio extension is appended
    #[arg(long, requires = "process")]
    template: Option<String>,
    #[arg(long, default_value = "bandcamp.cookies.txt")]
    cookies: PathBuf,
}
impl Args {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.user.is_empty()
                && self
                    .user
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "--user must be a Bandcamp username"
        );
        ensure!(
            self.artist.as_ref().is_none_or(|a| !a.trim().is_empty()),
            "--artist must not be empty"
        );
        if let (Some(from), Some(to)) = (self.fromdt, self.todt) {
            ensure!(from <= to, "--fromdt must not be after --todt");
        }
        Ok(())
    }
    fn matches(&self, date: Option<NaiveDate>) -> bool {
        if self.fromdt.is_none() && self.todt.is_none() {
            return true;
        }
        date.is_some_and(|d| {
            self.fromdt.is_none_or(|from| d >= from) && self.todt.is_none_or(|to| d <= to)
        })
    }
}

fn initcap(value: &str) -> String {
    sanitizing::display(value)
}

fn date(value: &Value) -> Option<NaiveDate> {
    if let Some(n) = value.as_i64() {
        return DateTime::from_timestamp(n, 0).map(|d| d.date_naive());
    }
    let s = value.as_str()?;
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc).date_naive());
    }
    if let Ok(d) = DateTime::parse_from_rfc2822(s) {
        return Some(d.with_timezone(&Utc).date_naive());
    }
    if let Ok(d) = NaiveDateTime::parse_from_str(s, "%d %b %Y %H:%M:%S GMT") {
        return Some(d.date());
    }
    s.get(..10)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}
fn metadata_date(item: &Value, fields: &[&str]) -> Option<NaiveDate> {
    fields.iter().find_map(|key| date(&item[*key]))
}
fn text_id(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|v| v.to_string()))
}

#[derive(Clone, Debug)]
struct Release {
    key: String,
    artist: String,
    title: String,
    url: String,
    publish: Option<NaiveDate>,
    release_url: Option<String>,
    release_custom_host: Option<String>,
    private: bool,
}

fn release_custom_host(item: &Value) -> Option<String> {
    let hints = &item["url_hints"];
    if hints["custom_domain_verified"] != 1 && hints["custom_domain_verified"] != true {
        return None;
    }
    let host = hints["custom_domain"].as_str()?;
    let url = reqwest::Url::parse(item["item_url"].as_str()?).ok()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|h| h.eq_ignore_ascii_case(host)))
    .then(|| url.host_str().unwrap().to_owned())
}

fn release_page_url(item: &Value) -> Result<Option<String>> {
    let Some(raw) = item["item_url"].as_str().filter(|url| !url.is_empty()) else {
        return Ok(None);
    };
    let mut url = reqwest::Url::parse(raw).context("Invalid release-page URL")?;
    if !url
        .host_str()
        .is_some_and(|host| host == "bandcamp.com" || host.ends_with(".bandcamp.com"))
        && let Some(subdomain) = item["url_hints"]["subdomain"].as_str()
    {
        ensure!(
            !subdomain.is_empty()
                && subdomain.len() <= 63
                && !subdomain.starts_with('-')
                && !subdomain.ends_with('-')
                && subdomain
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "Invalid Bandcamp release subdomain"
        );
        url.set_host(Some(&format!("{subdomain}.bandcamp.com")))
            .context("Invalid Bandcamp release host")?;
        url.set_scheme("https")
            .map_err(|_| anyhow::anyhow!("Invalid release-page scheme"))?;
        url.set_port(None)
            .map_err(|_| anyhow::anyhow!("Invalid release-page port"))?;
        ensure!(
            net::allowed(&url, false),
            "Invalid canonical release-page origin"
        );
    }
    Ok(Some(url.into()))
}

fn capture_page(
    db: &Database,
    args: &Args,
    items: &[Value],
    urls: &Value,
    seen: &mut HashSet<String>,
    selected: &mut Vec<Release>,
) -> Result<()> {
    for item in items {
        shutdown::check()?;
        if selected.len() >= args.count.map(|n| n as usize).unwrap_or(usize::MAX) {
            break;
        }
        let key = item_key(item)?;
        if !seen.insert(key.clone()) || flagged(item) {
            continue;
        }
        if !sanitizing::artist_matches(
            args.artist.as_deref(),
            item["band_name"].as_str().unwrap_or("Unknown Artist"),
        ) {
            continue;
        }
        let sale = text_id(&item["sale_item_id"]).unwrap_or_default();
        let sale_key = format!("{}{sale}", item["sale_item_type"].as_str().unwrap_or(""));
        let Some(url) = urls[&sale_key].as_str().or_else(|| urls[&key].as_str()) else {
            continue;
        };
        ensure!(
            net::allowed(&reqwest::Url::parse(url)?, false),
            "Unexpected download-page origin"
        );
        let purchase = metadata_date(item, &["purchased", "purchase_date"]);
        let publish = metadata_date(
            item,
            &[
                "release_date",
                "album_release_date",
                "publish_date",
                "released",
            ],
        );
        if !args.matches(purchase) || publish.is_some_and(|d| d > Utc::now().date_naive()) {
            continue;
        }
        let release = Release {
            key,
            artist: sanitizing::artist_display(
                item["band_name"].as_str().unwrap_or("Unknown Artist"),
            ),
            title: item["item_title"].as_str().unwrap_or("Untitled").to_owned(),
            url: url.into(),
            publish,
            release_url: release_page_url(item)?,
            release_custom_host: release_custom_host(item),
            private: item["is_private"].as_bool().unwrap_or(false),
        };
        let (sanitized, catalog) = sanitizing::title(
            item["item_title"].as_str().unwrap_or("Untitled"),
            item["band_name"].as_str().unwrap_or("Unknown Artist"),
        );
        db.query("INSERT INTO releases(user,item_key,artist,release_title,publish_date,purchase_date,download_url,release_url)
            VALUES(?,?,?,?,NULLIF(?,''),NULLIF(?,''),?,NULLIF(?,'')) ON CONFLICT(user,item_key) DO UPDATE SET
            artist=excluded.artist,release_title=excluded.release_title,
            publish_date=COALESCE(excluded.publish_date,releases.publish_date),
            purchase_date=COALESCE(excluded.purchase_date,releases.purchase_date),download_url=excluded.download_url,
            release_url=COALESCE(excluded.release_url,releases.release_url)",
            &[&args.user,&release.key,&release.artist,&release.title,&publish.map(|d| d.to_string()).unwrap_or_default(),&purchase.map(|d| d.to_string()).unwrap_or_default(),url,release.release_url.as_deref().unwrap_or_default()])?;
        db.query("UPDATE releases SET sanitized_title=?,catalog_num=COALESCE(NULLIF(CASE WHEN lower(trim(catalog_num))='none' THEN '' ELSE catalog_num END,''),NULLIF(?,'')) WHERE user=? AND item_key=?", &[&sanitized,catalog.as_deref().unwrap_or_default(),&args.user,&release.key])?;
        let release = Release {
            title: sanitized,
            ..release
        };
        if !db.completed(&args.user, &release.key, args.force)? {
            selected.push(release);
        }
    }
    Ok(())
}

fn capture(db: &Database, args: &Args, net: &Network) -> Result<Vec<Release>> {
    let data = net.page(&format!("https://bandcamp.com/{}", args.user))?;
    let fan = data.pointer("/fan_data/fan_id").context("Missing fan ID")?;
    if let Some(artist) = &args.artist {
        let page = net.collection_search(
            &json!({"fan_id":fan,"search_key":artist,"search_type":"collection"}),
        )?;
        let items = page["tralbums"]
            .as_array()
            .context("Missing collection search tralbums")?;
        let mut selected = Vec::new();
        capture_page(
            db,
            args,
            items,
            &page["redownload_urls"],
            &mut HashSet::new(),
            &mut selected,
        )?;
        eprintln!(
            "Captured {} eligible releases from artist search ({} results).",
            selected.len(),
            items.len()
        );
        return Ok(selected);
    }
    let cache = data
        .pointer("/item_cache/collection")
        .and_then(Value::as_object)
        .context("Missing collection cache")?;
    let mut initial: Vec<_> = cache.values().cloned().collect();
    initial.sort_by(|a, b| b["token"].as_str().cmp(&a["token"].as_str()));
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    capture_page(
        db,
        args,
        &initial,
        &data["collection_data"]["redownload_urls"],
        &mut seen,
        &mut selected,
    )?;
    let mut token = data["collection_data"]["last_token"]
        .as_str()
        .map(str::to_owned);
    let total = data["collection_data"]["item_count"]
        .as_u64()
        .context("Missing collection count")?;
    let mut more = (seen.len() as u64) < total;
    while more && selected.len() < args.count.map(|n| n as usize).unwrap_or(usize::MAX) {
        let previous = token.clone().context("Missing pagination token")?;
        let page =
            net.collection(&json!({"fan_id":fan,"count":100,"older_than_token":previous}))?;
        let before = seen.len();
        capture_page(
            db,
            args,
            page["items"]
                .as_array()
                .context("Missing pagination items")?,
            &page["redownload_urls"],
            &mut seen,
            &mut selected,
        )?;
        more = page["more_available"]
            .as_bool()
            .context("Missing more_available")?;
        token = page["last_token"].as_str().map(str::to_owned);
        if more
            && selected.len() < args.count.map(|n| n as usize).unwrap_or(usize::MAX)
            && (before == seen.len() || token.as_deref() == Some(previous.as_str()))
        {
            bail!("Collection pagination stalled");
        }
    }
    eprintln!(
        "Captured {} eligible releases (inspected {}).",
        selected.len(),
        seen.len()
    );
    Ok(selected)
}

fn process(
    db: &Database,
    args: &Args,
    releases: &[Release],
    mut fetch: impl FnMut(&Release) -> Result<download::Downloaded>,
) -> Result<()> {
    let mut failures = 0;
    for (index, release) in releases.iter().enumerate() {
        if shutdown::requested() {
            break;
        }
        if db.completed(&args.user, &release.key, args.force)? {
            continue;
        }
        db.increment_attempts(&args.user, &release.key)?;
        eprintln!(
            "[{}/{}] {} - {}",
            index + 1,
            releases.len(),
            release.artist,
            release.title
        );
        failures += record_result(db, args, release, fetch(release))?;
    }

    shutdown::check()?;
    ensure!(
        failures == 0,
        "{failures} releases failed; their completion dates were not changed"
    );
    Ok(())
}
fn record_result(
    db: &Database,
    args: &Args,
    release: &Release,
    result: Result<download::Downloaded>,
) -> Result<usize> {
    match result {
        Ok(done) => {
            db.query("UPDATE releases SET download_date=?,publish_date=COALESCE(NULLIF(?,''),publish_date),format=?,file_path=?,last_error=NULL,process_date=NULL,process_error=NULL,processed_path=NULL,genre_fallback=NULL WHERE user=? AND item_key=?",
                &[&Utc::now().to_rfc3339(),&done.publish.map(|d| d.to_string()).unwrap_or_default(),&done.format,&done.path.to_string_lossy(),&args.user,&release.key])?;
            Ok(0)
        }
        Err(_) if shutdown::requested() => Ok(0),
        Err(error) => {
            // HTTP errors have signed URLs removed at the request boundary. Keep the useful cause.
            let message = format!("{error:#}");
            eprintln!(
                "  Failed: {} - {}: {message}",
                release.artist, release.title
            );
            db.query(
                "UPDATE releases SET last_error=? WHERE user=? AND item_key=?",
                &[&message, &args.user, &release.key],
            )?;
            Ok(1)
        }
    }
}

fn process_parallel<P: Send>(
    db: &Database,
    args: &Args,
    releases: &[Release],
    mut prepare: impl FnMut(&Release) -> Result<P>,
    fetch: impl Fn(P) -> Result<download::Downloaded> + Sync,
) -> Result<()> {
    use std::{collections::VecDeque, thread};
    let limit = args.parallel.context("Missing --parallel limit")? as usize;
    let mut failures = 0;
    thread::scope(|scope| -> Result<()> {
        let mut active = VecDeque::new();
        let fetch = &fetch;
        for (index, release) in releases.iter().enumerate() {
            if shutdown::requested() {
                break;
            }
            if db.completed(&args.user, &release.key, args.force)? {
                continue;
            }
            db.increment_attempts(&args.user, &release.key)?;
            eprintln!(
                "[{}/{}] {} - {}",
                index + 1,
                releases.len(),
                release.artist,
                release.title
            );
            match prepare(release) {
                Ok(prepared) => active.push_back((release, scope.spawn(move || fetch(prepared)))),
                Err(error) => failures += record_result(db, args, release, Err(error))?,
            }
            if active.len() == limit {
                let (release, task) = active.pop_front().unwrap();
                let result = task
                    .join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("aria2c worker panicked")));
                failures += record_result(db, args, release, result)?;
            }
        }
        for (release, task) in active {
            let result = task
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("aria2c worker panicked")));
            failures += record_result(db, args, release, result)?;
        }
        Ok(())
    })?;
    shutdown::check()?;
    ensure!(
        failures == 0,
        "{failures} releases failed; their completion dates were not changed"
    );
    Ok(())
}

fn run(args: Args) -> Result<()> {
    let started = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    args.validate()?;
    if args.resources {
        return resources::run(&args);
    }
    sanitizing::load_artist_map()?;
    if args.process {
        return processing::run(&args);
    }
    fs::create_dir_all(&args.output).context("Create output directory")?;
    let db = Database::open(
        args.output
            .join("collection.sqlite3")
            .to_str()
            .context("Output path is not UTF-8")?,
    )?;
    let result = (|| {
        if args.parallel.is_some() {
            aria2::cleanup_stale()?;
            aria2::check()?;
        }
        let net = Network::new(&args.cookies)?;
        let releases = capture(&db, &args, &net)?;
        if args.parallel.is_some() {
            process_parallel(
                &db,
                &args,
                &releases,
                |release| {
                    let prepared = download::prepare(&net, &args, release, &db)?;
                    net.pace()?;
                    Ok(prepared)
                },
                |prepared| aria2::fetch(&args, prepared),
            )
        } else {
            process(&db, &args, &releases, |release| {
                download::fetch(&net, &args, release, &db)
            })
        }
    })();
    // Dump metadata even when a request fails; token-bearing links are omitted.
    if args.summary {
        db.dump(&mut io::stdout().lock(), Some((&args.user, &started)))?;
    }
    result
}
fn main() {
    let args = Args::parse();
    let result = shutdown::install().and_then(|()| run(args));
    if shutdown::requested() {
        eprintln!(
            "Termination complete; active transfers cleaned up and committed history preserved."
        );
        if let Err(error) = result {
            eprintln!("Stopped: {error:#}");
        }
        std::process::exit(shutdown::exit_code());
    }
    if let Err(error) = result {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
