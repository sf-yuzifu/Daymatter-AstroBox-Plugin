use std::fmt;

pub const MAX_NAME_LENGTH: usize = 50;
pub fn event_occurrence(event:&super::state::EventData,today:&str) -> Option<(String,Option<i32>)> {
    if event.extensions.get("calendar").and_then(|v|v.as_str()) == Some("lunar") {return super::lunar::occurrence(event,today);}
    occurrence(&event.time,event.extensions.get("repeat").and_then(|v|v.as_str()).unwrap_or("none"),today)
}

pub fn occurrence(date: &str, repeat: &str, today: &str) -> Option<(String, Option<i32>)> {
    use chrono::{Datelike, NaiveDate};
    let original = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    if repeat != "yearly" { return Some((date.to_string(), None)); }
    let today = NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    let mut year = original.year().max(today.year());
    let make = |year| NaiveDate::from_ymd_opt(year, original.month(), original.day())
        .or_else(|| NaiveDate::from_ymd_opt(year, 2, 28));
    let mut target = make(year)?;
    if target < today { year += 1; target = make(year)?; }
    if year > 9999 { return None; }
    Some((target.format("%Y-%m-%d").to_string(), Some(year - original.year())))
}

pub fn trim_name(name: &str) -> &str {
    name.trim_matches(|c: char| matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' |
        '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}'))
}

pub fn validate_name(name: &str, previous: Option<&str>) -> Result<(), &'static str> {
    if trim_name(name).is_empty() { return Err("事件名称不能为空"); }
    if previous == Some(name) { return Ok(()); }
    if trim_name(name).chars().count() > MAX_NAME_LENGTH { return Err("事件名称最多 50 个 Unicode 字符"); }
    Ok(())
}

pub fn normalized_name(name: &str, previous: Option<&str>) -> String {
    if previous == Some(name) { name.to_string() } else { trim_name(name).to_string() }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn 名称码点与空白边界() {
        for name in ["中".repeat(50), "😀".repeat(50), "e\u{0301}".repeat(25)] {
            assert!(validate_name(&format!("\u{feff}\u{3000}{}\u{00a0}", name), None).is_ok());
        }
        for name in ["中".repeat(51), "😀".repeat(51), "e\u{0301}".repeat(26)] {
            assert!(validate_name(&name, None).is_err());
        }
        assert_eq!(validate_name("\u{feff}\u{3000}\t", None), Err("事件名称不能为空"));
        assert_eq!(normalized_name(" a  b\u{0085} ", None), "a  b\u{0085}");
    }

    #[test]
    fn 旧长名称只允许原样保留() {
        let old = format!(" {} ", "旧".repeat(60));
        assert!(validate_name(&old, Some(&old)).is_ok());
        assert_eq!(normalized_name(&old, Some(&old)), old);
        assert!(validate_name(&"新".repeat(51), Some(&old)).is_err());
        assert_eq!(normalized_name(" 新名称 ", Some(&old)), "新名称");
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DateValidationError {
    Empty,
    InvalidFormat,
    InvalidYear,
    InvalidMonth,
    InvalidDay,
}

impl fmt::Display for DateValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateValidationError::Empty => write!(f, "时间不能为空"),
            DateValidationError::InvalidFormat => write!(f, "时间格式必须为 YYYY-MM-DD"),
            DateValidationError::InvalidYear => write!(f, "年份必须在 1-9999 之间"),
            DateValidationError::InvalidMonth => write!(f, "月份必须在 1-12 之间"),
            DateValidationError::InvalidDay => write!(f, "日期无效"),
        }
    }
}

pub fn validate_date_format(date_str: &str) -> Result<(), DateValidationError> {
    if date_str.is_empty() {
        return Err(DateValidationError::Empty);
    }

    if date_str.len() != 10 || !date_str.is_ascii() {
        return Err(DateValidationError::InvalidFormat);
    }

    let chars: Vec<char> = date_str.chars().collect();

    if chars[4] != '-' || chars[7] != '-' {
        return Err(DateValidationError::InvalidFormat);
    }

    let year_str = &date_str[0..4];
    let month_str = &date_str[5..7];
    let day_str = &date_str[8..10];

    let year: u32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    let month: u32 = match month_str.parse() {
        Ok(m) => m,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    let day: u32 = match day_str.parse() {
        Ok(d) => d,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    if year < 1 || year > 9999 {
        return Err(DateValidationError::InvalidYear);
    }

    if month < 1 || month > 12 {
        return Err(DateValidationError::InvalidMonth);
    }

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return Err(DateValidationError::InvalidMonth),
    };

    if day < 1 || day > days_in_month {
        return Err(DateValidationError::InvalidDay);
    }

    Ok(())
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 接受合法日期() {
        for date in ["2026-10-06", "2024-02-29", "2000-02-29", "2026-12-31"] {
            assert!(validate_date_format(date).is_ok(), "{} 应通过", date);
        }
    }

    #[test]
    fn 拒绝空值与格式错误() {
        assert_eq!(validate_date_format(""), Err(DateValidationError::Empty));
        assert_eq!(validate_date_format("2026-1-6"), Err(DateValidationError::InvalidFormat));
        assert_eq!(validate_date_format("2026/10/06"), Err(DateValidationError::InvalidFormat));
        assert_eq!(validate_date_format("2026-10-0a"), Err(DateValidationError::InvalidFormat));
    }

    #[test]
    fn 拒绝不存在的日历日期() {
        assert_eq!(validate_date_format("2026-02-30"), Err(DateValidationError::InvalidDay));
        assert_eq!(validate_date_format("2026-04-31"), Err(DateValidationError::InvalidDay));
        assert_eq!(validate_date_format("2023-02-29"), Err(DateValidationError::InvalidDay));
        assert_eq!(validate_date_format("2026-13-01"), Err(DateValidationError::InvalidMonth));
        assert_eq!(validate_date_format("2026-00-10"), Err(DateValidationError::InvalidMonth));
        assert_eq!(validate_date_format("2026-10-00"), Err(DateValidationError::InvalidDay));
    }

    #[test]
    fn 错误文案可读() {
        assert_eq!(DateValidationError::Empty.to_string(), "时间不能为空");
        assert_eq!(DateValidationError::InvalidDay.to_string(), "日期无效");
        assert!(DateValidationError::InvalidFormat.to_string().contains("YYYY-MM-DD"));
    }

    #[test]
    fn 闰年判定符合规则() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2026));
    }
}
