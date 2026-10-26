//! Performer and remix credit handling adapted from mcfunl.
use super::Tags;
pub fn artist_names(value: &str) -> Vec<String> {
    parsed_names(value, true)
}
fn parsed_names(value: &str, expand: bool) -> Vec<String> {
    let mut pending = vec![value];
    let mut names: Vec<String> = Vec::new();
    while let Some(value) = pending.pop() {
        let value = value.trim();
        if expand && let Some(mapped) = crate::sanitizing::mapped_artists(value) {
            for name in mapped {
                if !names.iter().any(|old| old.eq_ignore_ascii_case(name)) {
                    names.push(name.clone());
                }
            }
            continue;
        }
        let lower = value.to_ascii_lowercase();
        let split = [
            "|",
            " / ",
            " (feat.",
            " (ft.",
            " (feat ",
            " (featuring ",
            " (with ",
            " feat.",
            " ft.",
            " feat ",
            " featuring ",
            " with ",
            " remixed by ",
            " remix by ",
        ]
        .into_iter()
        .filter_map(|marker| {
            // Lowercase 'with' is a credit marker; preserve proper names such as Man With No Name.
            let text = if marker.contains("with") {
                value
            } else {
                &lower
            };
            text.find(marker).map(|position| (position, marker))
        })
        .min_by_key(|(position, _)| *position);
        if let Some((position, marker)) = split {
            let mut guest = value[position + marker.len()..].trim();
            if marker.starts_with(" (") {
                guest = guest.strip_suffix(')').unwrap_or(guest);
            }
            pending.push(guest);
            pending.push(&value[..position]);
        } else if !value.is_empty() && !names.iter().any(|name| name.eq_ignore_ascii_case(value)) {
            names.push(value.to_owned());
        }
    }
    names
}
fn remix_artists(value: &str) -> Vec<String> {
    let mut versions = value
        .split(['(', '['])
        .skip(1)
        .map(|part| part.split([')', ']']).next().unwrap_or(""))
        .collect::<Vec<_>>();
    if let Some((_, version)) = value.rsplit_once(" - ") {
        versions.push(version);
    }
    let lower = value.to_ascii_lowercase();
    for marker in ["remixed by ", "remix by "] {
        if let Some(position) = lower.find(marker) {
            versions.push(value[position..].split([')', ']']).next().unwrap_or(""));
        }
    }
    let mut names = Vec::new();
    for version in versions {
        let version = version.trim();
        let lower = version.to_ascii_lowercase();
        let credit = if lower.starts_with("remixed by ") {
            Some(&version[11..])
        } else if lower.starts_with("remix by ") {
            Some(&version[9..])
        } else if lower.ends_with(" remix") {
            Some(&version[..version.len() - 6])
        } else {
            None
        };
        if let Some(credit) = credit {
            let mut credit = credit.trim();
            while let Some((artist, description)) = credit.rsplit_once(' ')
                && ["extended", "club", "radio", "dub", "vocal", "instrumental"]
                    .contains(&description.to_ascii_lowercase().as_str())
            {
                credit = artist.trim();
            }
            if [
                "",
                "original",
                "extended",
                "club",
                "radio",
                "dub",
                "vocal",
                "instrumental",
                "vip",
            ]
            .contains(&credit.to_ascii_lowercase().as_str())
            {
                continue;
            }
            let credit = credit
                .strip_suffix("'s")
                .or_else(|| credit.strip_suffix("’s"))
                .or_else(|| credit.strip_suffix(['\'', '’']))
                .unwrap_or(credit);
            for name in artist_names(credit) {
                if !names
                    .iter()
                    .any(|old: &String| old.eq_ignore_ascii_case(&name))
                {
                    names.push(name);
                }
            }
        }
    }
    names
}
pub fn normalize_artists(tags: &mut Tags) {
    let mut remixers = tags
        .get("REMIXER")
        .into_iter()
        .flatten()
        .flat_map(|value| artist_names(value))
        .collect::<Vec<_>>();
    for value in tags
        .get("TITLE")
        .into_iter()
        .flatten()
        .chain(tags.get("ARTIST").into_iter().flatten())
    {
        for name in remix_artists(value) {
            if !remixers.iter().any(|old| old.eq_ignore_ascii_case(&name)) {
                remixers.push(name);
            }
        }
    }
    for key in [
        "ARTIST",
        "TRACKARTIST",
        "PERFORMER",
        "ARTISTSORT",
        "ALBUMARTIST",
        "ALBUM_ARTIST",
        "ALBUMARTISTSORT",
        "REMIXER",
    ] {
        if let Some(values) = tags.get_mut(key) {
            let mut names = Vec::new();
            for value in values.iter() {
                let mut value = value.as_str();
                for remixer in &remixers {
                    let suffix = format!(" & {remixer}");
                    let prefix = format!("{remixer} & ");
                    if value
                        .to_ascii_lowercase()
                        .ends_with(&suffix.to_ascii_lowercase())
                    {
                        value = &value[..value.len() - suffix.len()];
                    } else if value
                        .to_ascii_lowercase()
                        .starts_with(&prefix.to_ascii_lowercase())
                    {
                        value = &value[prefix.len()..];
                    }
                }
                for name in artist_names(value) {
                    if !names
                        .iter()
                        .any(|old: &String| old.eq_ignore_ascii_case(&name))
                    {
                        names.push(name);
                    }
                }
            }
            if matches!(key, "ALBUMARTIST" | "ALBUM_ARTIST" | "ALBUMARTISTSORT") {
                names.truncate(1);
            }
            *values = names;
        }
    }
    if !remixers.is_empty() {
        for key in ["ARTIST", "TRACKARTIST"] {
            if let Some(names) = tags.get_mut(key)
                && !names.is_empty()
            {
                for name in &remixers {
                    if !names.iter().any(|old| old.eq_ignore_ascii_case(name)) {
                        names.push(name.clone());
                    }
                }
            }
        }
        tags.insert("REMIXER".into(), remixers);
    }
}

/// Decode explicit credit separators without adding inferred artists/remixers.
pub fn split_fields(tags: &mut Tags) {
    for key in [
        "ARTIST",
        "TRACKARTIST",
        "PERFORMER",
        "ARTISTSORT",
        "ALBUMARTIST",
        "ALBUM_ARTIST",
        "ALBUMARTISTSORT",
        "REMIXER",
    ] {
        if let Some(values) = tags.get_mut(key) {
            let mut names = Vec::new();
            for value in values.iter() {
                for name in parsed_names(value, false) {
                    if !names
                        .iter()
                        .any(|old: &String| old.eq_ignore_ascii_case(&name))
                    {
                        names.push(name);
                    }
                }
            }
            *values = names;
        }
    }
}
pub fn append_names(names: &mut Vec<String>, value: &str) {
    for name in artist_names(value) {
        if !names.iter().any(|old| old.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
}

// Adapted from mcfunl's conservative album-prefix/featured-credit cleanup.
pub fn album_credit(text: &str, artist: &str) -> (String, Vec<String>) {
    fn prefix<'a>(text: &'a str, value: &str) -> Option<&'a str> {
        text.get(..value.len())
            .filter(|part| part.eq_ignore_ascii_case(value))
            .map(|_| &text[value.len()..])
    }
    fn featuring(text: &str) -> Option<&str> {
        ["featuring ", "feat. ", "ft. ", "feat ", "ft "]
            .into_iter()
            .find_map(|marker| prefix(text, marker))
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = text.as_str();
    let mut guests = Vec::new();
    if !artist.is_empty() {
        while let Some(rest) = prefix(title, artist) {
            if let Some(rest) = rest.strip_prefix(" - ") {
                title = rest.trim();
                continue;
            }
            let rest = rest.trim_start();
            if prefix(rest, artist).is_some() {
                title = rest;
                continue;
            }
            if let Some(credit) = rest.strip_prefix('(').and_then(featuring)
                && let Some((guest, remaining)) = credit.split_once(')')
                && !guest.trim().is_empty()
                && !remaining.trim().is_empty()
            {
                append_names(&mut guests, guest);
                title = remaining
                    .trim()
                    .strip_prefix("- ")
                    .unwrap_or(remaining.trim());
            } else if let Some(credit) = featuring(rest)
                && let Some((guest, remaining)) = credit.split_once(" - ")
                && !guest.trim().is_empty()
                && !remaining.trim().is_empty()
            {
                append_names(&mut guests, guest);
                title = remaining.trim();
            }
            break;
        }
    }
    if title.is_empty() {
        (text, Vec::new())
    } else {
        (title.into(), guests)
    }
}
