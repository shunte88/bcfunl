//! Public release tags and the artist sidebar location, without geocoding guesses.
use crate::db::Database;
use anyhow::Result;
use scraper::{Html, Selector};

pub(crate) fn country(value: &str) -> Option<&'static str> {
    let alias = match value.trim().to_ascii_lowercase().as_str() {
        "uk" | "u.k." | "england" | "scotland" | "wales" | "northern ireland" => Some("GB"),
        "usa" | "u.s.a." | "united states of america" => Some("US"),
        "south korea" => Some("KR"),
        "north korea" => Some("KP"),
        "russia" => Some("RU"),
        "vietnam" => Some("VN"),
        "taiwan" => Some("TW"),
        "bolivia" => Some("BO"),
        "venezuela" => Some("VE"),
        "iran" => Some("IR"),
        "syria" => Some("SY"),
        "laos" => Some("LA"),
        "tanzania" => Some("TZ"),
        "moldova" => Some("MD"),
        "czech republic" => Some("CZ"),
        _ => None,
    };
    alias.or_else(|| {
        include_str!("../data/countries.tsv")
            .lines()
            .find_map(|line| {
                let mut fields = line.split('\t');
                let code = fields.next()?;
                (code.eq_ignore_ascii_case(value.trim())
                    || fields.any(|name| name.eq_ignore_ascii_case(value.trim())))
                .then_some(code)
            })
    })
}

fn extract(html: &str) -> (Option<String>, Option<&'static str>) {
    let doc = Html::parse_document(html);
    let location = doc
        .select(&Selector::parse("#band-name-location .location").unwrap())
        .next()
        .map(|e| {
            e.text()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let parts: Vec<_> = location
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    // A city alone is ambiguous; only explicit countries or full US state names qualify.
    let country_iso = if parts.len() > 1
        && parts
            .last()
            .is_some_and(|part| part.eq_ignore_ascii_case("Georgia"))
    {
        None // Georgia can mean a country or a US state.
    } else {
        parts.last().and_then(|part| country(part)).or_else(|| {
            let states = "Alabama|Alaska|Arizona|Arkansas|California|Colorado|Connecticut|Delaware|Florida|Georgia|Hawaii|Idaho|Illinois|Indiana|Iowa|Kansas|Kentucky|Louisiana|Maine|Maryland|Massachusetts|Michigan|Minnesota|Mississippi|Missouri|Montana|Nebraska|Nevada|New Hampshire|New Jersey|New Mexico|New York|North Carolina|North Dakota|Ohio|Oklahoma|Oregon|Pennsylvania|Rhode Island|South Carolina|South Dakota|Tennessee|Texas|Utah|Vermont|Virginia|Washington|West Virginia|Wisconsin|Wyoming|District of Columbia";
            let us_state = parts.len() > 1 && parts.last().is_some_and(|part| {
                states.split('|').any(|state| state.eq_ignore_ascii_case(part))
            });
            us_state.then_some("US")
        })
    };
    let mut tags: Vec<String> = Vec::new();
    for element in doc.select(&Selector::parse(".tralbum-tags a.tag").unwrap()) {
        let tag = element
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let mut classes = element.value().classes();
        let href = element.value().attr("href").unwrap_or_default();
        let is_location = classes.any(|c| matches!(c, "location" | "location-tag" | "geotag"))
            || href.contains("/location/")
            || href.contains("geoname_id=")
            || tag.eq_ignore_ascii_case(&location)
            || parts.iter().any(|part| tag.eq_ignore_ascii_case(part))
            || country(&tag).is_some();
        if !tag.is_empty() && !is_location && !tags.iter().any(|old| old.eq_ignore_ascii_case(&tag))
        {
            tags.push(tag);
        }
    }
    let genre = (!tags.is_empty()).then(|| tags.join(";"));
    (genre, country_iso)
}

pub fn store(db: &Database, user: &str, key: &str, html: &str) -> Result<()> {
    let (genre, country_iso) = extract(html);
    db.query(
        "UPDATE releases SET genre=NULLIF(?,''),country_iso=NULLIF(?, '') WHERE user=? AND item_key=?",
        &[genre.as_deref().unwrap_or_default(), country_iso.unwrap_or_default(), user, key],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genre_tags_exclude_locations_and_keep_order_without_duplicates() {
        let html = r#"<div id="band-name-location"><span class="location">Belfast, UK</span></div>
        <div class="tralbum-tags"><a class="tag">electronic</a><a class="tag">Belfast</a>
        <a class="tag">ambient</a><a class="tag">Electronic</a><a class="tag">United Kingdom</a>
        <a class="tag location-tag">London</a></div><a class="tag">unrelated</a>"#;
        assert_eq!(
            extract(html),
            (Some("electronic;ambient".into()), Some("GB"))
        );
        assert_eq!(
            extract("<div id='band-name-location'><span class='location'>London</span></div>"),
            (None, None)
        );
        assert_eq!(
            extract(
                "<div id='band-name-location'><span class='location'>Portland, Oregon</span></div>"
            ),
            (None, Some("US"))
        );
        assert_eq!(
            extract(
                "<div id='band-name-location'><span class='location'>Atlanta, Georgia</span></div>"
            ),
            (None, None)
        );
        assert_eq!(
            extract(
                "<div id='band-name-location'><span class='location'>London, United Kingdom</span></div><div class='tralbum-tags'><a class='tag'>London</a><a class='tag'>post-punk</a></div>"
            ),
            (Some("post-punk".into()), Some("GB"))
        );
        assert_eq!(
            extract("<div class='tralbum-tags'><a class='tag'>ambient</a></div>"),
            (Some("ambient".into()), None)
        );
        assert_eq!(country("Germany"), Some("DE"));
        assert_eq!(country("Japan"), Some("JP"));
        assert_eq!(country("unknown"), None);
    }

    #[test]
    fn metadata_updates_preserve_completion_history() {
        let db = Database::open(":memory:").unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,attempts) VALUES('u','k','Artist','Title','https://bandcamp.com/download','completed',3)", &[]).unwrap();
        store(&db, "u", "k", "<div id='band-name-location'><span class='location'>Tokyo, Japan</span></div><div class='tralbum-tags'><a class='tag'>ambient</a><a class='tag'>Tokyo</a></div>").unwrap();
        assert_eq!(
            db.query(
                "SELECT genre,country_iso,download_date,attempts FROM releases",
                &[]
            )
            .unwrap()[0],
            vec![
                Some("ambient".into()),
                Some("JP".into()),
                Some("completed".into()),
                Some("3".into())
            ]
        );
        assert!(db.completed("u", "k", false).unwrap());
    }
}
