//! Timestamp normalization shared by every adapter.
//!
//! Local histories store time three ways: an ISO-8601 string, epoch seconds, and epoch milliseconds.
//! All three normalize to one UTC ISO string ending in `Z`, byte-identical to the reference Python
//! extractor, because the record identity hash covers the timestamp.

/// Convert a JSON timestamp value into epoch seconds.
///
/// A number larger than 100_000_000_000 is milliseconds; Cursor stores time that way.
pub fn epoch_seconds(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => {
            let seconds = number.as_f64()?;
            Some(if seconds > 100_000_000_000.0 { seconds / 1000.0 } else { seconds })
        }
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Ok(number) = trimmed.parse::<f64>() {
                return Some(if number > 100_000_000_000.0 { number / 1000.0 } else { number });
            }
            parse_iso(trimmed)
        }
        _ => None,
    }
}

/// Normalize any accepted timestamp into `YYYY-MM-DDTHH:MM:SS[.ffffff]Z`.
pub fn iso_timestamp(value: &serde_json::Value) -> Option<String> {
    epoch_seconds(value).map(format_epoch)
}

/// True when `value` is at or after `cutoff`. A record without a usable timestamp is outside every
/// bounded window, matching the reference extractor.
pub fn in_window(value: &serde_json::Value, cutoff: Option<f64>) -> bool {
    match cutoff {
        None => true,
        Some(cutoff) => epoch_seconds(value).map_or(false, |seconds| seconds >= cutoff),
    }
}

fn parse_iso(text: &str) -> Option<f64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i64 = text[0..4].parse().ok()?;
    let month: i64 = text[5..7].parse().ok()?;
    let day: i64 = text[8..10].parse().ok()?;
    if !matches!(bytes[10], b'T' | b' ') {
        return None;
    }
    let hour: i64 = text[11..13].parse().ok()?;
    let minute: i64 = text[14..16].parse().ok()?;
    let second: i64 = text[17..19].parse().ok()?;
    let mut rest = &text[19..];
    let mut fraction = 0.0f64;
    if let Some(stripped) = rest.strip_prefix('.') {
        let digits: String = stripped.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        fraction = format!("0.{digits}").parse().ok()?;
        rest = &rest[1 + digits.len()..];
    }
    let mut offset_seconds = 0i64;
    if !rest.is_empty() && rest != "Z" && rest != "z" {
        let sign = match rest.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let body = &rest[1..];
        let (offset_hour, offset_minute) = if let Some((left, right)) = body.split_once(':') {
            (left.parse::<i64>().ok()?, right.parse::<i64>().ok()?)
        } else if body.len() == 4 {
            (body[0..2].parse().ok()?, body[2..4].parse().ok()?)
        } else {
            (body.parse::<i64>().ok()?, 0)
        };
        offset_seconds = sign * (offset_hour * 3600 + offset_minute * 60);
    }
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Some(seconds as f64 + fraction)
}

/// Render epoch seconds the way `datetime.fromtimestamp(value, timezone.utc).isoformat()` does:
/// the fractional field appears only when it is non-zero, and always as six digits.
pub fn format_epoch(epoch: f64) -> String {
    let total_micros = (epoch * 1_000_000.0).round() as i64;
    let (mut seconds, mut micros) = (total_micros.div_euclid(1_000_000), total_micros.rem_euclid(1_000_000));
    if micros < 0 {
        micros += 1_000_000;
        seconds -= 1;
    }
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (day_seconds / 3600, (day_seconds % 3600) / 60, day_seconds % 60);
    if micros == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{micros:06}Z")
    }
}

/// Howard Hinnant's civil calendar algorithms, days relative to 1970-01-01.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 { month_prime + 3 } else { month_prime - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_iso_string_normalizes_to_a_utc_z_string() {
        assert_eq!(
            iso_timestamp(&json!("2026-09-01T13:00:00.000Z")).unwrap(),
            "2026-09-01T13:00:00Z"
        );
    }

    #[test]
    fn an_offset_is_converted_to_utc() {
        assert_eq!(
            iso_timestamp(&json!("2026-09-01T23:00:00+10:00")).unwrap(),
            "2026-09-01T13:00:00Z"
        );
    }

    #[test]
    fn epoch_milliseconds_are_detected_and_scaled() {
        assert_eq!(
            iso_timestamp(&json!(1_788_267_600_000i64)).unwrap(),
            "2026-09-01T13:00:00Z"
        );
    }

    #[test]
    fn a_sub_second_fraction_is_kept_at_six_digits() {
        assert_eq!(
            iso_timestamp(&json!("2026-09-01T13:00:00.5Z")).unwrap(),
            "2026-09-01T13:00:00.500000Z"
        );
    }

    #[test]
    fn a_record_without_a_timestamp_is_outside_a_bounded_window() {
        assert!(!in_window(&json!(null), Some(0.0)));
        assert!(in_window(&json!(null), None));
    }

    #[test]
    fn civil_round_trip_holds_across_a_leap_day() {
        let epoch = 1_709_164_800.0; // 2024-02-29T00:00:00Z
        assert_eq!(format_epoch(epoch), "2024-02-29T00:00:00Z");
        assert_eq!(epoch_seconds(&json!("2024-02-29T00:00:00Z")).unwrap(), epoch);
    }
}
