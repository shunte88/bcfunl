//! Shared Discogs title search and index identity.
pub(crate) fn edition_note(note: &str) -> bool {
    let note = note
        .to_ascii_lowercase()
        .replace("re-issued", "reissued")
        .replace("re-mastered", "remastered");
    let mut separated = String::new();
    let mut previous = ' ';
    for c in note.chars() {
        if (previous.is_ascii_alphabetic() && c.is_ascii_digit())
            || (previous.is_ascii_digit() && c.is_ascii_alphabetic())
        {
            separated.push(' ');
        }
        separated.push(c);
        previous = c;
    }
    let words: Vec<_> = separated
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
                "digital" | "digitally" | "edition" | "version" | "in" | "sde" | "super"
                | "deluxe" | "expanded" => {}
                _ => return false,
            }
        }
    }
    year && kind
}

pub(crate) fn search_title(raw: &str) -> String {
    let mut title = raw.trim();
    loop {
        let mut cut = None;
        for (open, close) in [('(', ')'), ('[', ']')] {
            if title.ends_with(close)
                && let Some(pos) = title.rfind(open)
                && edition_note(&title[pos + 1..title.len() - 1])
            {
                cut = Some(pos);
            }
        }
        if cut.is_none() {
            for (pos, c) in title.char_indices() {
                if c.is_whitespace() && edition_note(title[pos..].trim()) {
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

pub(crate) fn lookup_title(value: &str) -> String {
    search_title(value)
        .replace(['(', ')', '[', ']'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub(crate) fn title_key(value: &str) -> String {
    let title = lookup_title(value);
    let mut words: Vec<_> = title.split_whitespace().collect();
    for i in 0..words.len().saturating_sub(1) {
        if ["vol.", "vol", "volume"]
            .iter()
            .any(|marker| words[i].eq_ignore_ascii_case(marker))
            && words[i + 1].chars().all(|c| c.is_ascii_digit())
        {
            words[i] = "volume";
        }
    }
    words.join(" ").to_ascii_lowercase()
}
pub(crate) fn same(a: &str, b: &str) -> bool {
    let a = title_key(a);
    !a.is_empty() && a == title_key(b)
}

pub(crate) fn master_title(raw: &str) -> String {
    let mut title = search_title(raw);
    let lower = raw.to_ascii_lowercase();
    let deluxe = [
        "[deluxe",
        "(deluxe",
        "[super deluxe",
        "(super deluxe",
        "[sde",
        "(sde",
        "[expanded",
        "(expanded",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    while let Some((start, close)) =
        [('[', ']'), ('(', ')')]
            .into_iter()
            .find_map(|(open, close)| {
                title
                    .ends_with(close)
                    .then(|| title.rfind(open).map(|start| (start, close)))
                    .flatten()
            })
    {
        let note = title[start + 1..title.len() - close.len_utf8()]
            .trim()
            .to_ascii_lowercase();
        let edition = edition_note(&note)
            || [
                "super deluxe edition",
                "deluxe edition",
                "deluxe",
                "sde",
                "expanded edition",
            ]
            .contains(&note.as_str());
        let component = deluxe
            && [
                "b-sides and remixes",
                "b sides and remixes",
                "live sessions",
                "bonus tracks",
                "bonus disc",
            ]
            .contains(&note.as_str());
        if !(edition || component) || title[..start].trim().is_empty() {
            break;
        }
        title = title[..start].trim().to_owned();
    }
    search_title(&title)
}
