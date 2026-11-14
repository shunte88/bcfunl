//! Shared Discogs title search and index identity.
pub(crate) fn search_title(raw: &str) -> String {
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
