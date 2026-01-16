use std::fmt;

#[derive(Debug)]
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
