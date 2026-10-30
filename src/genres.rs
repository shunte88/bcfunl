//! Local genre hygiene around the shared VCW expansion table.
use std::sync::OnceLock;

fn identity(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn acceptable(value: &str, artists: &[String]) -> bool {
    crate::sanitizing::mapped_artists(value).is_none()
        && !value.chars().any(char::is_numeric)
        && value.chars().filter(|c| c.is_alphabetic()).count() > 1
        && !artists
            .iter()
            .any(|artist| identity(artist) == identity(value))
}
pub(crate) fn normalize<I, S>(values: I, artists: &[String]) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    static MAP: OnceLock<vcw_metadata::Genres> = OnceLock::new();
    let inputs: Vec<String> = values
        .into_iter()
        .flat_map(|value| {
            value
                .as_ref()
                .split(';')
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|value| acceptable(value, artists))
        .collect();
    let mut result: Vec<String> = Vec::new();
    for value in MAP
        .get_or_init(vcw_metadata::Genres::builtin)
        .normalize_all(&inputs)
    {
        if !acceptable(&value, artists) {
            continue;
        }
        let value = crate::initcap(&value.to_lowercase());
        if !result.iter().any(|old| old.eq_ignore_ascii_case(&value)) {
            result.push(value);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_numeric_short_and_artist_tags_before_and_after_expansion() {
        assert_eq!(
            normalize(
                [
                    "123;80s;techno2026;１２３;A;!;µ;The Black Dog;the-black-dog;Brock Van Wey;AMBIENT;ambient"
                ],
                &["The Black Dog".into()]
            ),
            ["Ambient"]
        );
        let values = normalize(["post-punk;acid house;Unmapped Words"], &[]);
        assert!(values.contains(&"Post-Punk".into()));
        assert!(values.contains(&"Acid House".into()));
        assert!(values.contains(&"House".into()));
        assert!(values.contains(&"Unmapped Words".into()));
    }
}
