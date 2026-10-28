//! Host local clock for HOMResultsData.Reset825698b8's GetLocalTime.
use chrono::{Datelike, Timelike};
pub(crate) fn now() -> [u16; 6] {
    let time = chrono::Local::now();
    [
        time.year() as u16,
        time.month() as u16,
        time.day() as u16,
        time.hour() as u16,
        time.minute() as u16,
        time.second() as u16,
    ]
}
