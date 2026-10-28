//! Shared display cleanup for captured metadata and downstream tags.
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};

struct ArtistMap {
    names: BTreeMap<String, Vec<String>>,
    title_phrases: Vec<(String, Vec<String>)>,
}
static ARTISTS: OnceLock<ArtistMap> = OnceLock::new();
fn artist_key(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            if word.eq_ignore_ascii_case("and") || word == "&" {
                "&".into()
            } else {
                word.to_lowercase()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn parse_artist_map(text: &str) -> Result<ArtistMap> {
    let raw: BTreeMap<String, Vec<String>> =
        serde_yaml::from_str(text).context("Parse artistmap.yaml")?;
    let mut title_phrases: Vec<_> = raw
        .iter()
        .map(|(key, names)| {
            (
                key.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase(),
                names.iter().map(|n| n.trim().to_owned()).collect(),
            )
        })
        .collect();
    title_phrases.sort_by_key(|(key, _)| std::cmp::Reverse(key.len()));
    let mut map = BTreeMap::new();
    for (source, values) in raw {
        let key = artist_key(&source);
        ensure!(
            !key.is_empty() && !values.is_empty(),
            "Artist mapping requires a source and at least one name"
        );
        let mut names: Vec<String> = Vec::new();
        for name in values {
            let name = name.trim();
            ensure!(
                !name.is_empty() && !name.chars().any(char::is_control),
                "Artist mapping contains an empty name or control character"
            );
            ensure!(
                !names.iter().any(|n| artist_key(n) == artist_key(name)),
                "Duplicate name in artist mapping: {source}"
            );
            names.push(name.to_owned());
        }
        ensure!(
            map.insert(key, names).is_none(),
            "Duplicate case-insensitive artist mapping: {source}"
        );
    }
    let expanded: Vec<_> = map
        .values()
        .filter(|names| names.len() > 1)
        .map(|names| (artist_key(&names.join(" / ")), names.clone()))
        .collect();
    for (key, names) in expanded {
        if let Some(existing) = map.get(&key) {
            ensure!(*existing == names, "Conflicting joined artist mapping");
        } else {
            map.insert(key, names);
        }
    }
    let mut canonical = BTreeMap::new();
    for names in map.values() {
        for name in names {
            if let Some(previous) = canonical.insert(artist_key(name), name.clone()) {
                ensure!(
                    previous == *name,
                    "Conflicting canonical artist spelling: {name}"
                );
            }
        }
    }
    for (key, name) in canonical {
        if let Some(values) = map.get(&key) {
            ensure!(
                values == &[name] || values.iter().any(|value| artist_key(value) == key),
                "Artist map output is also a conflicting expansion key"
            );
        } else {
            map.insert(key, vec![name]);
        }
    }
    Ok(ArtistMap {
        names: map,
        title_phrases,
    })
}
pub fn load_artist_map() -> Result<()> {
    let path = Path::new("data/artistmap.yaml");
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            include_str!("../data/artistmap.yaml").to_owned()
        }
        Err(error) => return Err(error).context("Read data/artistmap.yaml"),
    };
    let map = parse_artist_map(&text)?;
    let _ = ARTISTS.set(map);
    Ok(())
}
pub fn mapped_artists(value: &str) -> Option<&'static [String]> {
    ARTISTS
        .get_or_init(|| {
            parse_artist_map(include_str!("../data/artistmap.yaml"))
                .expect("valid bundled artist map")
        })
        .names
        .get(&artist_key(value))
        .map(Vec::as_slice)
}
pub fn artist_display(value: &str) -> String {
    mapped_artists(value)
        .map(|names| names.join(" / "))
        .unwrap_or_else(|| display(value))
}

pub fn title_credits(value: &str) -> (String, Vec<String>) {
    let map = ARTISTS.get_or_init(|| {
        parse_artist_map(include_str!("../data/artistmap.yaml")).expect("valid bundled artist map")
    });
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut remaining = value.as_str();
    let mut title = String::new();
    let mut credits: Vec<String> = Vec::new();
    let mut boundary = true;
    while !remaining.is_empty() {
        let found = boundary
            .then(|| {
                map.title_phrases.iter().find(|(phrase, _)| {
                    remaining
                        .get(..phrase.len())
                        .is_some_and(|part| part.eq_ignore_ascii_case(phrase))
                        && remaining[phrase.len()..]
                            .chars()
                            .next()
                            .is_none_or(|c| !c.is_alphanumeric() && c != '\'' && c != '’')
                })
            })
            .flatten();
        if let Some((phrase, names)) = found {
            for name in names {
                if !credits.iter().any(|old| old.eq_ignore_ascii_case(name)) {
                    credits.push(name.clone());
                }
            }
            remaining = &remaining[phrase.len()..];
            boundary = true;
        } else {
            let c = remaining.chars().next().unwrap();
            title.push(c);
            remaining = &remaining[c.len_utf8()..];
            boundary = !c.is_alphanumeric() && c != '\'' && c != '’';
        }
    }
    if !credits.is_empty() {
        title = title
            .replace("( )", "")
            .replace("[ ]", "")
            .replace("()", "")
            .replace("[]", "");
        title = title
            .trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | ':' | ',' | ';'))
            .to_owned();
    }
    (title_display(&title), credits)
}

pub fn title_display(value: &str) -> String {
    let formatted = display(value);
    let mut names: Vec<_> = ARTISTS
        .get_or_init(|| {
            parse_artist_map(include_str!("../data/artistmap.yaml"))
                .expect("valid bundled artist map")
        })
        .names
        .values()
        .flatten()
        .collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();
    let mut out = String::new();
    let mut remaining = formatted.as_str();
    let mut boundary = true;
    while !remaining.is_empty() {
        let found = boundary
            .then(|| {
                names.iter().find(|name| {
                    remaining
                        .get(..name.len())
                        .is_some_and(|part| part.eq_ignore_ascii_case(name))
                        && remaining[name.len()..]
                            .chars()
                            .next()
                            .is_none_or(|c| !c.is_alphanumeric() && c != '\'' && c != '’')
                })
            })
            .flatten();
        if let Some(name) = found {
            out.push_str(name);
            remaining = &remaining[name.len()..];
            boundary = false;
        } else {
            let c = remaining.chars().next().unwrap();
            out.push(c);
            boundary = !c.is_alphanumeric() && c != '\'' && c != '’';
            remaining = &remaining[c.len_utf8()..];
        }
    }
    out
}

pub fn display(value: &str) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let mut start = true;
    let preserve = !value.bytes().any(|c| c.is_ascii_lowercase());
    for c in value.chars() {
        if c.is_alphabetic() {
            out.push(if preserve || start {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            });
            start = false;
        } else {
            out.push(c);
            start = !c.is_numeric();
        }
    }
    let suffixes = ["s", "t", "m", "re", "ve", "ll", "d"];
    let mut result = String::new();
    for part in out.split_inclusive(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’') {
        let mut part = part.to_owned();
        for quote in ['\'', '’'] {
            if let Some((head, tail)) = part.split_once(quote) {
                let tail_word = tail.trim_end_matches(|c: char| !c.is_alphanumeric());
                if suffixes.iter().any(|s| tail_word.eq_ignore_ascii_case(s)) {
                    part = format!(
                        "{head}{quote}{}{}",
                        tail_word.to_ascii_lowercase(),
                        &tail[tail_word.len()..]
                    );
                }
            }
        }
        let word = part.trim_end_matches(|c: char| !c.is_alphanumeric());
        if let Some(canonical) = include_str!("../data/forceupper.txt")
            .lines()
            .map(str::trim)
            .find(|s| !s.is_empty() && s.eq_ignore_ascii_case(word))
        {
            part = format!("{canonical}{}", &part[word.len()..]);
        }
        result.push_str(&part);
    }
    result
}
pub fn artist_matches(filter: Option<&str>, artist: &str) -> bool {
    filter.is_none_or(|f| {
        artist_display(artist)
            .to_lowercase()
            .contains(&artist_display(f).to_lowercase())
    })
}
fn catalog_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    token.len() <= 32
        && token.chars().any(|c| c.is_ascii_alphabetic())
        && token.chars().any(|c| c.is_ascii_digit())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-./".contains(c))
        && !lower.ends_with("bpm")
        && ![
            "cd", "disc", "disk", "vol", "lp", "bpm", "dvd", "sacd", "mp3",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn colon_catalog(token: &str) -> bool {
    // Bare prefixes need a stricter label-code shape than bracketed hints.
    let letters = token.bytes().take_while(u8::is_ascii_alphabetic).count();
    let digits = token[letters..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    let suffix = &token[letters + digits..];
    catalog_token(token)
        && (2..=12).contains(&letters)
        && (3..=8).contains(&digits)
        && suffix.len() <= 3
        && suffix.bytes().all(|c| c.is_ascii_alphabetic())
}

fn ombra_catalog(value: &str) -> Option<(String, &str)> {
    for prefix in ["ombra international", "ombra intl"] {
        let Some(head) = value.get(..prefix.len()) else {
            continue;
        };
        if !head.eq_ignore_ascii_case(prefix) {
            continue;
        }
        let tail = value[prefix.len()..].trim_start();
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=6).contains(&digits) {
            continue;
        }
        let rest = &tail[digits..];
        if rest
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace() && c != ':' && c != '-')
        {
            continue;
        }
        return Some((format!("OMBRA INTL {}", &tail[..digits]), rest));
    }
    None
}

pub fn title(raw: &str, artist: &str) -> (String, Option<String>) {
    let mut text = raw.trim();
    let prefix = artist.trim();
    if !prefix.is_empty()
        && text
            .get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        && text.get(prefix.len()..).is_some_and(|tail| {
            tail.starts_with(|c: char| c.is_whitespace() || c == '(' || c == ':')
        })
        && let Some((_, tail)) = text[prefix.len()..].split_once(" - ")
    {
        text = tail.trim();
    }
    text = text.trim_matches(['\'', '"', '‘', '’', '“', '”']).trim();
    let mut catalog = None;
    if let Some((code, rest)) = ombra_catalog(text) {
        let remaining = rest.trim().trim_start_matches([':', '-']).trim();
        if remaining.is_empty() {
            return (
                format!(
                    "Ombra International {}",
                    code.strip_prefix("OMBRA INTL ").unwrap()
                ),
                Some(code),
            );
        }
        catalog = Some(code);
        text = remaining;
    }
    for (open, close) in [('(', ')'), ('[', ']')] {
        if text.ends_with(close)
            && let Some(pos) = text.rfind(open)
        {
            let token = &text[pos + 1..text.len() - 1];
            if let Some((code, rest)) = ombra_catalog(token)
                && rest.trim().is_empty()
                && catalog.as_ref().is_none_or(|existing| existing == &code)
            {
                if text[..pos].trim().is_empty() {
                    return (
                        format!(
                            "Ombra International {}",
                            code.strip_prefix("OMBRA INTL ").unwrap()
                        ),
                        Some(code),
                    );
                }
                catalog = Some(code);
                text = text[..pos].trim_end();
            } else if catalog_token(token) && !text[..pos].trim().is_empty() {
                catalog = Some(token.to_ascii_uppercase());
                text = text[..pos].trim_end();
            }
        }
    }
    let leading = if text.starts_with('[') {
        text.find(']')
            .map(|end| (&text[1..end], text[end + 1..].trim(), false))
    } else {
        text.split_once(':')
            .map(|(token, rest)| (token, rest.trim(), true))
    };
    if let Some((token, rest, false)) = leading
        && let Some((code, tail)) = ombra_catalog(token)
        && tail.trim().is_empty()
        && !rest.is_empty()
        && catalog.as_ref().is_none_or(|existing| existing == &code)
    {
        return (title_display(rest), Some(code));
    }
    if let Some((token, rest, bare)) = leading
        && !rest.is_empty()
        && if bare {
            colon_catalog(token)
        } else {
            catalog_token(token)
        }
        && catalog
            .as_ref()
            .is_none_or(|existing| token.eq_ignore_ascii_case(existing))
    {
        catalog = Some(token.to_ascii_uppercase());
        text = rest;
    }
    (
        title_display(text.trim_matches(['\'', '"', '‘', '’', '“', '”']).trim()),
        catalog,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ombra_catalog_titles_remain_meaningful_and_idempotent() {
        for raw in [
            "Ombra Intl 006",
            "OMBRA INTL006",
            "Ombra International 006",
            "[Ombra INTL 006]",
        ] {
            let expected = (
                "Ombra International 006".into(),
                Some("OMBRA INTL 006".into()),
            );
            assert_eq!(title(raw, "Artist"), expected);
            assert_eq!(title(&expected.0, "Artist"), expected);
        }
        for raw in [
            "Ombra INTL022: Dilation & Truth",
            "Dilation & Truth [Ombra INTL 022]",
            "[Ombra INTL 022] Dilation & Truth",
        ] {
            assert_eq!(
                title(raw, "Artist"),
                ("Dilation & Truth".into(), Some("OMBRA INTL 022".into()))
            );
        }
        for raw in [
            "Ombra Intl Remix",
            "Ombra Intl 006Remix",
            "Ombra Intl 1234567",
            "Other Ombra Intl 006",
        ] {
            assert_eq!(title(raw, "Artist").1, None);
        }
    }

    #[test]
    fn leading_catalogs_and_artist_prefixes_preserve_subtitles() {
        for (raw, artist, expected, catalog) in [
            (
                "V\u{200b}/\u{200b}A - [Af005z] Voiceless Z",
                "V\u{200b}/\u{200b}A",
                "Voiceless Z",
                "AF005Z",
            ),
            ("µ-Ziq - [Af015] D Funk EP", "µ-Ziq", "D Funk EP", "AF015"),
            ("V/A - [Af005y] Voiceless Y", "V/A", "Voiceless Y", "AF005Y"),
            (
                "Brainwaltzera - [Af007] Outdives EP",
                "Brainwaltzera",
                "Outdives EP",
                "AF007",
            ),
            (
                "Morphology - [Af020] Collective Memory EP",
                "Morphology",
                "Collective Memory EP",
                "AF020",
            ),
            (
                "Various Artists - Hnrbox003: Upwards At 33 1/3 Degrees - Höga Nord Rekords Singles Collection Vol.3",
                "Various Artists",
                "Upwards At 33 1/3 Degrees - Höga Nord Rekords Singles Collection Vol.3",
                "HNRBOX003",
            ),
            ("[af015] D Funk EP", "µ-Ziq", "D Funk EP", "AF015"),
        ] {
            assert_eq!(
                title(raw, artist),
                (expected.into(), Some(catalog.into())),
                "{raw}"
            );
        }
        for (raw, artist, expected) in [
            ("Artist - Title - Subtitle", "Artist", "Title - Subtitle"),
            ("Title - Subtitle", "Artist", "Title - Subtitle"),
            ("AC - DC - Title - Subtitle", "AC - DC", "Title - Subtitle"),
        ] {
            assert_eq!(title(raw, artist), (expected.into(), None));
        }
        for raw in [
            "1984: A Story",
            "Chapter 003: A Story",
            "Part1: A Story",
            "[Deluxe] Title",
            "[2026] Title",
            "[CD1] Title",
            "[AF015]",
            "Hnrbox003:",
            "µ003: Title",
            "[AF015] Title [OTHER123]",
        ] {
            let (cleaned, catalog) = title(raw, "Artist");
            if raw == "[AF015] Title [OTHER123]" {
                assert_eq!(catalog.as_deref(), Some("OTHER123"));
                assert!(cleaned.starts_with("[Af015]"));
            } else {
                assert_eq!(catalog, None, "{raw}");
                assert!(!cleaned.is_empty());
            }
        }
    }

    #[test]
    fn contractions_acronyms_and_catalogs() {
        for (raw, expected) in [
            ("the xx", "The XX"),
            ("it's i'm I'M It'S", "It's I'm I'm It's"),
            ("o'rourke", "O'Rourke"),
            ("d'angelo", "D'Angelo"),
            ("dj bp usa", "DJ BP USA"),
        ] {
            assert_eq!(display(raw), expected);
        }
        assert_eq!(
            title(
                "David Holmes Ft. Raven Violet - 'Blind On A Galloping Horse'",
                "David Holmes"
            ),
            ("Blind On A Galloping Horse".into(), None)
        );
        assert_eq!(
            title(
                "David Holmes (Featuring Raven Violet) The Blind On A Galloping Horse Remixes - Yeah X3 (Hvn74012r)   ",
                "David Holmes"
            ),
            ("Yeah X3".into(), Some("HVN74012R".into()))
        );
        assert_eq!(
            title("This Is The Title [catalo546] ", "Artist"),
            ("This Is The Title".into(), Some("CATALO546".into()))
        );
        for raw in [
            "Title (Remixes Vol. 2)",
            "Title [Deluxe]",
            "Title (2026)",
            "Title (CD1)",
            "Title (120BPM)",
        ] {
            assert_eq!(title(raw, "Artist").1, None, "{raw}");
        }
        assert!(artist_matches(Some("the xx"), "The XX"));
        assert!(!artist_matches(Some("Holmes"), "The XX"));
    }
}

#[cfg(test)]
mod artist_map_tests {
    use super::*;
    #[test]
    fn yaml_map_expands_preserves_case_and_rejects_conflicts() {
        let map = parse_artist_map("alias:\n  - bvdub\n  - Brock Van Wey\n").unwrap();
        assert_eq!(map.names["alias"], ["bvdub", "Brock Van Wey"]);
        assert_eq!(map.names["bvdub"], ["bvdub"]);
        assert_eq!(
            map.names["bvdub / brock van wey"],
            ["bvdub", "Brock Van Wey"]
        );
        for bad in [
            "alias: []",
            "alias: []\nALIAS: [Other]",
            "alias: [Name, name]",
            "a: [bvdub]\nb: [Bvdub]",
            "alias: [\"\"]",
            "alias: wrong",
            "a: [b]\nb: [c]",
        ] {
            assert!(parse_artist_map(bad).is_err(), "{bad}");
        }
        assert_eq!(
            artist_display("BVDUB . BROCK VAN WEY"),
            "bvdub / Brock Van Wey"
        );
        assert_eq!(artist_display("Bvdub"), "bvdub");
        assert_eq!(
            title_display(
                "henry mccullough andrew weatherall remixes (gold & silver rsd editions)"
            ),
            "Henry McCullough Andrew Weatherall Remixes (Gold & Silver RSD Editions)"
        );
        assert_eq!(title_display("The BVDUB Remix"), "The bvdub Remix");
        assert_eq!(title_display("bvdubbed"), "Bvdubbed");
    }
}

#[cfg(test)]
mod conjunction_tests {
    use super::*;
    #[test]
    fn and_ampersand_keys_share_a_flat_self_named_expansion() {
        let map=parse_artist_map("Martin Stephenson & The Daintees:\n  - Martin Stephenson And The Daintees\n  - Martin Stephenson\n  - The Daintees\n").unwrap();
        assert_eq!(
            artist_key("Martin Stephenson AND The Daintees"),
            artist_key("Martin Stephenson & The Daintees")
        );
        assert_eq!(
            map.names[&artist_key("Martin Stephenson And The Daintees")],
            [
                "Martin Stephenson And The Daintees",
                "Martin Stephenson",
                "The Daintees"
            ]
        );
        assert_ne!(artist_key("Anderson"), artist_key("&erson"));
        assert!(parse_artist_map("A And B: [A]\nA & B: [B]").is_err());
        assert!(parse_artist_map("a: [b]\nb: [a]").is_err());
    }
}

// Only callers with confirmed past single-track evidence may remove these labels.
pub(crate) fn remove_prerelease(value: &str) -> String {
    let mut text = value.to_owned();
    for marker in [
        "(pre-release)",
        "[pre-release]",
        "(prerelease)",
        "[prerelease]",
        "(pre release)",
        "[pre release]",
    ] {
        while let Some(pos) = text.to_ascii_lowercase().find(marker) {
            text.replace_range(pos..pos + marker.len(), "");
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
