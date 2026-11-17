use chrono::NaiveDateTime;

use crate::gm::LossyConvert;

const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];

/// A size as a file manager shows it, in steps of 1000 like Finder:
/// `512 B`, `1.2 KB`, `3.4 MB`. Always English.
pub fn size_text(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }

    let mut value: f32 = bytes.lossy_convert();
    let mut unit = "B";

    for next in UNITS {
        if value < 1000.0 {
            break;
        }
        value /= 1000.0;
        unit = next;
    }

    // 999.96 KB would round up to a 4 digit number, show it as the next
    // whole hundred the way the eye expects.
    if value >= 99.95 {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.1} {unit}")
    }
}

/// A date as the date column shows it: `6 Oct 2026 14:05`. Always
/// English and 24 hours, whatever the system locale.
pub fn date_text(at: NaiveDateTime) -> String {
    at.format("%-d %b %Y %H:%M").to_string()
}

#[cfg(test)]
mod test {
    use chrono::NaiveDate;

    use super::{date_text, size_text};

    #[test]
    fn sizes_step_by_a_thousand() {
        assert_eq!(size_text(0), "0 B");
        assert_eq!(size_text(999), "999 B");
        assert_eq!(size_text(1000), "1.0 KB");
        assert_eq!(size_text(1234), "1.2 KB");
        assert_eq!(size_text(3_400_000), "3.4 MB");
        assert_eq!(size_text(150_000_000), "150 MB");
        assert_eq!(size_text(2_000_000_000), "2.0 GB");
        assert_eq!(size_text(999_999), "1000 KB");
    }

    #[test]
    fn the_date_has_no_leading_zero_on_the_day() {
        let at = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap().and_hms_opt(14, 5, 0).unwrap();
        assert_eq!(date_text(at), "6 Oct 2026 14:05");

        let at = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        assert_eq!(date_text(at), "31 Jan 2025 00:00");
    }
}
