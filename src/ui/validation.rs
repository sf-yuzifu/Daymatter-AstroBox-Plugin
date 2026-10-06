use std::fmt;

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

    if date_str.len() != 10 {
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
