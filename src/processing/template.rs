use anyhow::{Result, bail, ensure};
use std::{collections::BTreeMap, path::PathBuf};

pub const DEFAULT: &str =
    "{artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}";
pub type Values = BTreeMap<&'static str, String>;

pub fn safe(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if out.len() + c.len_utf8() > 180 {
            break;
        }
        out.push(if c.is_control() || "/\\:*?\"<>|".contains(c) {
            '_'
        } else {
            c
        });
    }
    let out = out.trim().trim_matches('.').trim();
    if out.is_empty() {
        "Unknown".into()
    } else {
        out.into()
    }
}

fn tokens(text: &str, values: &Values) -> Result<(String, bool)> {
    let mut result = String::new();
    let mut missing = false;
    let mut rest = text;
    while let Some(pos) = rest.find('{') {
        ensure!(!rest[..pos].contains('}'), "Unmatched template brace");
        result.push_str(&rest[..pos]);
        rest = &rest[pos + 1..];
        let end = rest
            .find('}')
            .ok_or_else(|| anyhow::anyhow!("Unmatched template brace"))?;
        let name = &rest[..end];
        let value = values
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown template field: {name}"))?;
        if value.is_empty() {
            missing = true;
        } else {
            result.push_str(&safe(value));
        }
        rest = &rest[end + 1..];
    }
    ensure!(!rest.contains('}'), "Unmatched template brace");
    result.push_str(rest);
    Ok((result, missing))
}

fn component(text: &str, values: &Values) -> Result<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&tokens(&rest[..start], values)?.0);
        let end = rest[start + 1..]
            .find(']')
            .ok_or_else(|| anyhow::anyhow!("Unmatched template bracket"))?
            + start
            + 1;
        let (group, missing) = tokens(&rest[start + 1..end], values)?;
        ensure!(
            !group.contains('['),
            "Nested template brackets are unsupported"
        );
        if !missing {
            out.push('[');
            out.push_str(&group);
            out.push(']');
        }
        rest = &rest[end + 1..];
    }
    ensure!(!rest.contains(']'), "Unmatched template bracket");
    out.push_str(&tokens(rest, values)?.0);
    Ok(out.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub fn fields() -> Values {
    [
        "artist",
        "albumartist",
        "trackartist",
        "performer",
        "artistsort",
        "albumartistsort",
        "remixer",
        "album",
        "country_iso",
        "catalog_num",
        "disc",
        "track",
        "title",
    ]
    .into_iter()
    .map(|key| (key, String::new()))
    .collect()
}

pub struct Template {
    text: String,
    release_parts: usize,
}
impl Template {
    pub fn new(text: &str) -> Result<Self> {
        ensure!(
            !text.starts_with('/') && !text.contains(['\\', '\0', '\n', '\r']),
            "Template must be a relative path"
        );
        let parts: Vec<_> = text.split('/').collect();
        ensure!(
            parts.len() >= 2
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && *part != "." && *part != ".."),
            "Template needs a release directory and filename without traversal"
        );
        let dynamic = [
            "{disc}",
            "{track}",
            "{title}",
            "{trackartist}",
            "{performer}",
            "{artistsort}",
            "{remixer}",
        ];
        let release_parts = parts[..parts.len() - 1]
            .iter()
            .position(|part| dynamic.iter().any(|key| part.contains(key)))
            .unwrap_or(parts.len() - 1);
        ensure!(
            release_parts > 0
                && parts[..release_parts]
                    .iter()
                    .any(|part| part.contains("{album}")),
            "Template needs an album directory before track-specific fields"
        );
        let known = fields();
        for part in &parts {
            component(part, &known)?;
        }
        Ok(Self {
            text: text.into(),
            release_parts,
        })
    }
    pub fn render(&self, values: &Values, extension: &str) -> Result<(PathBuf, PathBuf)> {
        ensure!(
            !extension.is_empty() && extension.bytes().all(|b| b.is_ascii_alphanumeric()),
            "Invalid audio extension"
        );
        let mut relative = PathBuf::new();
        let mut release = PathBuf::new();
        let count = self.text.split('/').count();
        for (index, part) in self.text.split('/').enumerate() {
            let mut rendered = component(part, values)?;
            ensure!(
                !rendered.is_empty()
                    && rendered != "."
                    && rendered != ".."
                    && !rendered.contains(['/', '\\']),
                "Template produced an unsafe or empty path component"
            );
            if index == count - 1 {
                rendered.push('.');
                rendered.push_str(extension);
            }
            ensure!(
                rendered.len() <= 240,
                "Template path component exceeds 240 bytes"
            );
            relative.push(rendered);
            if index + 1 == self.release_parts {
                release = relative.clone();
            }
        }
        if release.as_os_str().is_empty() {
            bail!("Template produced no release directory");
        }
        Ok((release, relative))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_fields_numbering_unicode_and_path_safety() {
        let mut v = fields();
        for (key, value) in [
            ("artist", "µ-Ziq"),
            ("album", "Proof"),
            ("disc", "01"),
            ("track", "02"),
            ("title", "../AC/DC"),
        ] {
            v.insert(key, value.into());
        }
        let t = Template::new(DEFAULT).unwrap();
        let (release, path) = t.render(&v, "flac").unwrap();
        assert_eq!(release, PathBuf::from("µ-Ziq/Proof"));
        assert_eq!(path, PathBuf::from("µ-Ziq/Proof/01-02 - _AC_DC.flac"));
        v.insert("country_iso", "GB".into());
        v.insert("catalog_num", "CAT01".into());
        assert_eq!(
            t.render(&v, "flac").unwrap().0,
            PathBuf::from("µ-Ziq/Proof [GB] [CAT01]")
        );
        for bad in [
            "/tmp/{album}/{title}",
            "../{album}/{title}",
            "{artist}/{oops}/{title}",
            "{album}/{title",
            "{artist}/{title}",
            "{album}/x\\y",
        ] {
            assert!(Template::new(bad).is_err(), "{bad}");
        }
        let multi = Template::new("{artist}/{album}/Disc {disc}/{track} - {title}").unwrap();
        assert_eq!(
            multi.render(&v, "wav").unwrap().0,
            PathBuf::from("µ-Ziq/Proof")
        );
    }
}
