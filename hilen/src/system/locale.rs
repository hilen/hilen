//! The user's preferred system locale.

use sys_locale::get_locale;

/// The system locale as a BCP 47 tag, like "en-US" or "ru-RU".
/// `None` when the OS reports nothing.
pub fn locale() -> Option<String> {
    get_locale()
}

/// The lowercase primary language subtag of the system locale,
/// like "en" or "ru". `None` when the OS reports nothing.
pub fn language_code() -> Option<String> {
    primary_subtag(&locale()?)
}

// Unix reports POSIX forms like "ru_RU.UTF-8", so '_' and '.' split too.
fn primary_subtag(locale: &str) -> Option<String> {
    let code = locale.split(['-', '_', '.']).next()?.trim();

    if code.is_empty() {
        return None;
    }

    Some(code.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{language_code, locale, primary_subtag};

    #[test]
    fn subtag_from_common_forms() {
        assert_eq!(primary_subtag("en-US").as_deref(), Some("en"));
        assert_eq!(primary_subtag("ru_RU.UTF-8").as_deref(), Some("ru"));
        assert_eq!(primary_subtag("LT").as_deref(), Some("lt"));
        assert_eq!(primary_subtag(""), None);
        assert_eq!(primary_subtag("-US"), None);
    }

    #[test]
    fn code_matches_locale() {
        let Some(tag) = locale() else {
            assert_eq!(language_code(), None);
            return;
        };

        assert!(tag.to_ascii_lowercase().starts_with(&language_code().unwrap()));
    }
}
