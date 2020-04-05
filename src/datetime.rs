use chrono::prelude::*;
use crate::error::Result;

const FMT_STR: &str = "%F %T%.3f %z";

pub fn now() -> DateTime<Local> {
    Local::now()
}

pub fn to_str(dt: DateTime<Local>) -> String {
    dt.format(FMT_STR).to_string()
}

pub fn from_str<T>(s: String) -> Result<DateTime<FixedOffset>> {
    Ok(DateTime::parse_from_str(&s, FMT_STR)?)
}
