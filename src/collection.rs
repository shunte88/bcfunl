use anyhow::{Context, Result, ensure};
use reqwest::header::HeaderValue;
use scraper::{Html, Selector};
use serde_json::Value;
// Cookies are scoped before each authenticated request; redirects are handled separately.
pub fn cookies(text: &str, path: &str, now: u64) -> Result<HeaderValue> {
    cookies_for(text,"bandcamp.com",path,now)?.context("No unexpired Bandcamp cookies match this request; export Bandcamp cookies in Netscape format")
}
pub fn cookies_for(text: &str, host: &str, path: &str, now: u64) -> Result<Option<HeaderValue>> {
    let mut pairs = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let line = line.strip_prefix("#HttpOnly_").unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        ensure!(
            fields.len() == 7,
            "Invalid Netscape cookie row {}",
            line_no + 1
        );
        let domain = fields[0].trim_start_matches('.');
        if domain != "bandcamp.com" && !domain.ends_with(".bandcamp.com") {
            continue;
        }
        if host != domain && !(fields[1] == "TRUE" && host.ends_with(&format!(".{domain}"))) {
            continue;
        }
        let expires: u64 = fields[4].parse().context("Invalid cookie expiry")?;
        let cookie_path = fields[2];
        let matches = path == cookie_path
            || (path.starts_with(cookie_path)
                && (cookie_path.ends_with('/') || path[cookie_path.len()..].starts_with('/')));
        if !matches || (expires != 0 && expires <= now) {
            continue;
        }
        ensure!(
            !fields[5].is_empty()
                && !fields[5].contains([';', '=', '\r', '\n'])
                && !fields[6].contains([';', '\r', '\n']),
            "Invalid cookie syntax"
        );
        pairs.push(format!("{}={}", fields[5], fields[6]));
    }
    if pairs.is_empty() {
        return Ok(None);
    }
    let mut header = HeaderValue::from_str(&pairs.join("; ")).context("Invalid cookie header")?;
    header.set_sensitive(true);
    Ok(Some(header))
}

pub fn pagedata(html: &str) -> Result<Value> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse("#pagedata").unwrap();
    let blob = doc
        .select(&selector)
        .next()
        .and_then(|e| e.value().attr("data-blob"))
        .context(
            "Missing collection pagedata; check login, privacy settings, or a site challenge",
        )?;
    serde_json::from_str(blob).context("Invalid collection pagedata JSON")
}

pub fn item_key(item: &Value) -> Result<String> {
    let kind = item["item_type"].as_str().context("Missing item_type")?;
    let id = item
        .get("item_id")
        .filter(|v| v.is_number() || v.is_string())
        .context("Missing item_id")?;
    Ok(format!(
        "{kind}{}",
        id.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| id.to_string())
    ))
}

// Bandcamp has used several preorder flag spellings across page types.
pub fn flagged(item: &Value) -> bool {
    [
        "is_preorder",
        "is_pre_order",
        "is_prerelease",
        "is_pre_release",
        "preorder",
        "pre_order",
    ]
    .iter()
    .any(|key| match &item[*key] {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_u64().is_some_and(|n| n > 0),
        Value::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    })
}
