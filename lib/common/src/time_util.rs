use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn unix_time() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn unix_timestamp_ms() -> u64 {
    unix_time().as_millis() as _
}

pub fn unix_utc_timestamp_ms() -> u64 {
    const UTC_OFFSET_MS: u64 = 3 * 3600 * 1000; // TODO: make it configurable or use some crate

    unix_timestamp_ms() - UTC_OFFSET_MS
}
