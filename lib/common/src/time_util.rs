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

pub fn current_time_in_ticks() -> u64 {
    const UNIX_EPOCH_TICKS: u64 = 621355968000000000;
    const TICKS_IN_MILLISECOND: u64 = 10_000;

    (unix_timestamp_ms() * TICKS_IN_MILLISECOND) + UNIX_EPOCH_TICKS
}
