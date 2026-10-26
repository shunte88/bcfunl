use crate::{
    collection::{cookies, cookies_for},
    shutdown,
};
use anyhow::{Context, Result, bail, ensure};
use rand::Rng;
use reqwest::{
    Client, Url,
    header::{COOKIE, LOCATION, USER_AGENT},
    redirect::Policy,
};
use serde_json::Value;
use std::{
    cell::Cell,
    fs,
    io::{self, Cursor, Read},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Runtime;

pub struct Network {
    client: Client,
    cookie_text: String,
    requested: Cell<bool>,
    runtime: Option<Runtime>,
}
// A response borrows the runtime so it is always dropped before runtime shutdown.
pub struct Body<'a> {
    response: reqwest::Response,
    runtime: &'a Runtime,
    chunk: Cursor<Vec<u8>>,
}
impl Body<'_> {
    pub fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }
    pub fn text(self) -> Result<String> {
        self.runtime
            .block_on(shutdown::until(self.response.text()))?
            .map_err(|e| e.without_url())
            .context("Read HTTP response")
    }
}
impl Read for Body<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            shutdown::check().map_err(io::Error::other)?;
            let n = self.chunk.read(buffer)?;
            if n > 0 {
                return Ok(n);
            }
            let chunk = self
                .runtime
                .block_on(shutdown::until(self.response.chunk()))
                .map_err(io::Error::other)?
                .map_err(|e| io::Error::other(e.without_url()))?;
            match chunk {
                Some(chunk) => self.chunk = Cursor::new(chunk.to_vec()),
                None => return Ok(0),
            }
        }
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        // OS DNS lookups cannot be canceled; do not block cleanup waiting for resolver threads.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
pub fn allowed(url: &Url, cdn: bool) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|h| {
            h == "bandcamp.com"
                || h.ends_with(".bandcamp.com")
                || (cdn && (h == "bcbits.com" || h.ends_with(".bcbits.com")))
        })
}
fn redirect_hop(source: &Url, status: reqwest::StatusCode, target: &Url) -> String {
    format!(
        "{} --{}--> {}",
        source.origin().ascii_serialization(),
        status.as_u16(),
        target.origin().ascii_serialization()
    )
}
fn allowed_release_host(url: &Url, custom_host: Option<&str>) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && custom_host.is_some_and(|host| url.host_str() == Some(host))
}
fn secure_request_url(mut url: Url, cdn: bool) -> Result<Url> {
    if url.scheme() == "http" && url.port_or_known_default() == Some(80) {
        let mut secure = url.clone();
        secure
            .set_scheme("https")
            .map_err(|_| anyhow::anyhow!("Cannot upgrade Bandcamp URL"))?;
        if allowed(&secure, false) {
            url = secure;
        }
    }
    ensure!(
        allowed(&url, cdn),
        "Rejected unexpected URL origin (scheme={}, host={}, port={})",
        url.scheme(),
        url.host_str().unwrap_or("missing"),
        url.port_or_known_default().unwrap_or(0)
    );
    Ok(url)
}
fn request_delay_ms() -> u64 {
    rand::thread_rng().gen_range(1100..=5000)
}
impl Network {
    pub(crate) fn pace(&self) -> Result<()> {
        shutdown::check()?;
        if self.requested.replace(true) {
            shutdown::pause(Duration::from_millis(request_delay_ms()))?;
        }
        Ok(())
    }
    pub fn new(path: &Path) -> Result<Self> {
        let cookie_text = fs::read_to_string(path).context("Read cookie export")?;
        cookies(
            &cookie_text,
            "/",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        )?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            cookie_text,
            requested: Cell::new(false),
            runtime: Some(runtime),
            client: Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(90))
                .redirect(Policy::none())
                .user_agent(concat!("bcfunl/", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }
    fn runtime(&self) -> &Runtime {
        self.runtime.as_ref().unwrap()
    }
    fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        self.runtime()
            .block_on(shutdown::until(async { request.send().await }))?
            .map_err(|e| e.without_url())
            .context("Bandcamp request")
    }
    pub fn get(&self, url: &str, cdn: bool) -> Result<Body<'_>> {
        self.get_scoped(url, cdn, None, false)
    }
    pub fn release_page(&self, url: &str, custom_host: Option<&str>) -> Result<Body<'_>> {
        self.get_scoped(url, false, custom_host, true)
    }
    fn get_request(&self, url: Url, cdn: bool, release_page: bool) -> reqwest::RequestBuilder {
        let request =
            self.client
                .get(url)
                .timeout(Duration::from_secs(if cdn { 1800 } else { 90 }));
        if release_page {
            // Bandcamp's custom-domain redirect behavior depends on a full browser user agent.
            request.header(
                USER_AGENT,
                concat!(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) ",
                    "Chrome/130.0.0.0 Safari/537.36 bcfunl/",
                    env!("CARGO_PKG_VERSION")
                ),
            )
        } else {
            request
        }
    }
    fn get_scoped(
        &self,
        url: &str,
        cdn: bool,
        custom_host: Option<&str>,
        release_page: bool,
    ) -> Result<Body<'_>> {
        let mut url = Url::parse(url).context("Invalid Bandcamp URL")?;
        let mut redirects = Vec::new();
        for _ in 0..6 {
            if !allowed_release_host(&url, custom_host) {
                url = secure_request_url(url, cdn)?;
            }
            let mut request = self.get_request(url.clone(), cdn, release_page);
            if let Some(cookie) = cookies_for(
                &self.cookie_text,
                url.host_str().unwrap_or(""),
                url.path(),
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            )? {
                request = request.header(COOKIE, cookie);
            }
            self.pace()?;
            let response = self.send(request)?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(LOCATION)
                    .context("Redirect missing Location")?
                    .to_str()?;
                let target = url.join(location).context("Invalid redirect")?;
                redirects.push(redirect_hop(&url, response.status(), &target));
                url = target;
                continue;
            }
            ensure!(
                response.status().is_success(),
                "Bandcamp returned HTTP {}",
                response.status()
            );
            return Ok(Body {
                response,
                runtime: self.runtime(),
                chunk: Cursor::new(Vec::new()),
            });
        }
        bail!("Too many Bandcamp redirects: {}", redirects.join("; "))
    }
    pub fn page(&self, url: &str) -> Result<Value> {
        crate::collection::pagedata(&self.get(url, false)?.text()?)
    }
    pub fn collection(&self, body: &Value) -> Result<Value> {
        self.collection_request("collection_items", body)
    }
    pub fn collection_search(&self, body: &Value) -> Result<Value> {
        self.collection_request("search_items", body)
    }
    fn collection_request(&self, endpoint: &str, body: &Value) -> Result<Value> {
        let path = format!("/api/fancollection/1/{endpoint}");
        self.pace()?;
        let request = self
            .client
            .post(format!("https://bandcamp.com{path}"))
            .header(
                COOKIE,
                cookies(
                    &self.cookie_text,
                    &path,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                )?,
            )
            .json(body);
        let response = self.send(request)?;
        ensure!(
            response.status().is_success(),
            "Collection pagination returned HTTP {}",
            response.status()
        );
        self.runtime()
            .block_on(shutdown::until(response.json()))?
            .map_err(|e| e.without_url())
            .context("Invalid pagination JSON")
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn browser_compatibility_header_is_limited_to_release_page_requests() {
        let root = crate::tests::tempdir();
        let path = root.join("fixture-cookies.txt");
        std::fs::write(&path, ".bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tfixture\n").unwrap();
        let network = super::Network::new(&path).unwrap();
        let url =
            reqwest::Url::parse("https://theblackdog.bandcamp.com/album/feral-grace").unwrap();
        let release = network
            .get_request(url.clone(), false, true)
            .build()
            .unwrap();
        let agent = release.headers()[reqwest::header::USER_AGENT]
            .to_str()
            .unwrap();
        assert!(
            agent.starts_with("Mozilla/5.0")
                && agent.contains("Chrome/130.")
                && agent.ends_with(concat!("bcfunl/", env!("CARGO_PKG_VERSION")))
        );
        assert!(release.headers().get(reqwest::header::COOKIE).is_none());
        for cdn in [false, true] {
            let ordinary = network
                .get_request(url.clone(), cdn, false)
                .build()
                .unwrap();
            // Client defaults are applied at send time; ordinary requests have no override.
            assert!(
                ordinary
                    .headers()
                    .get(reqwest::header::USER_AGENT)
                    .is_none()
            );
        }
        drop(network);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn redirect_diagnostics_include_origins_and_status_without_private_url_data() {
        let source = reqwest::Url::parse(
            "https://user:secret@theblackdog.bandcamp.com/album/private?sig=hidden",
        )
        .unwrap();
        let target =
            reqwest::Url::parse("http://duststoredigital.com/album/private?token=hidden").unwrap();
        assert_eq!(
            super::redirect_hop(&source, reqwest::StatusCode::MOVED_PERMANENTLY, &target),
            "https://theblackdog.bandcamp.com --301--> http://duststoredigital.com"
        );
    }
    #[test]
    fn release_redirect_scope_allows_only_the_exact_verified_custom_host() {
        use reqwest::Url;
        let host = Some("duststoredigital.com");
        assert!(super::allowed_release_host(
            &Url::parse("https://duststoredigital.com/album/feral-grace").unwrap(),
            host
        ));
        for bad in [
            "http://duststoredigital.com/album/test",
            "https://duststoredigital.com.evil.org/test",
            "https://other.example/test",
            "https://duststoredigital.com:8443/test",
            "https://user:pass@duststoredigital.com/test",
        ] {
            assert!(!super::allowed_release_host(
                &Url::parse(bad).unwrap(),
                host
            ));
        }
        assert!(!super::allowed_release_host(
            &Url::parse("https://duststoredigital.com/test").unwrap(),
            None
        ));
        assert!(
            crate::collection::cookies_for(
                ".bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tfixture\n",
                "duststoredigital.com",
                "/",
                0
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn upgrades_legacy_bandcamp_links_without_relaxing_origin_checks() {
        use reqwest::Url;
        let upgraded = super::secure_request_url(
            Url::parse("http://theblackdog.bandcamp.com/album/liber-eleven").unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(
            upgraded.as_str(),
            "https://theblackdog.bandcamp.com/album/liber-eleven"
        );
        for bad in [
            "http://bandcamp.com.evil.org/secret?token=private",
            "http://bandcamp.com:8080/x",
            "http://user:pass@bandcamp.com/x",
            "http://p4.bcbits.com/x",
        ] {
            let message = super::secure_request_url(Url::parse(bad).unwrap(), true)
                .unwrap_err()
                .to_string();
            assert!(!message.contains("private") && !message.contains("pass@"));
        }
        assert!(
            super::secure_request_url(Url::parse("https://p4.bcbits.com/x").unwrap(), true).is_ok()
        );
    }
    #[test]
    fn client_initialization_and_runtime_cleanup() {
        let root = crate::tests::tempdir();
        let path = root.join("fixture-cookies.txt");
        std::fs::write(&path, ".bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tfixture\n").unwrap();
        let network = super::Network::new(&path).unwrap();
        drop(network);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn requests_start_inside_the_runtime() {
        let root = crate::tests::tempdir();
        let path = root.join("fixture-cookies.txt");
        std::fs::write(&path, ".bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tfixture\n").unwrap();
        let network = super::Network::new(&path).unwrap();
        for method in [reqwest::Method::GET, reqwest::Method::POST] {
            // A closed local port exercises request startup without a server or real credentials.
            let request = network
                .client
                .request(method, "http://127.0.0.1:1")
                .timeout(std::time::Duration::from_millis(100));
            assert!(network.send(request).is_err());
        }
        drop(network);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn randomized_delay_stays_in_requested_bounds() {
        for _ in 0..1000 {
            assert!((1100..=5000).contains(&super::request_delay_ms()));
        }
    }
}
