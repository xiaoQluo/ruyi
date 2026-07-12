use std::thread;
use std::time::Duration;
/**
 * C FFI implementations backing `stdlib/time.ry`.
 *
 * Provides time-related functions via C ABI for use by Ruyi standard library.
 * All functions wrap Rust's standard time library functions.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix timestamp in seconds.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __time_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Returns the current Unix timestamp in milliseconds.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __time_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Sleeps for the specified number of seconds.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __time_sleep(seconds: f64) {
    let duration = Duration::from_secs_f64(seconds);
    thread::sleep(duration);
}

/// Formats a Unix timestamp into a string.
///
/// Returns a simple ISO 8601 format string: "YYYY-MM-DD HH:MM:SS"
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __time_format(timestamp: i64) -> *mut i8 {
    use std::alloc::{alloc, Layout};

    // Simple timestamp to date conversion
    // This is a basic implementation - a full implementation would handle timezones
    let seconds = timestamp as u64;
    let days = seconds / 86400;
    let remaining = seconds % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let secs = remaining % 60;

    // Calculate year, month, day (simplified calendar calculation)
    let mut year = 1970;
    let mut day_count = days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if day_count < days_in_year {
            break;
        }
        day_count -= days_in_year;
        year += 1;
    }

    let month_days = get_month_days(year, is_leap_year(year));
    let mut month = 0;
    let mut day = day_count;

    for (i, &days_in_month) in month_days.iter().enumerate() {
        let days_in_month_u64 = days_in_month as u64;
        if day < days_in_month_u64 {
            month = i + 1;
            break;
        }
        day -= days_in_month_u64;
    }

    // Format as "YYYY-MM-DD HH:MM:SS"
    let formatted = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month,
        day + 1,
        hours,
        minutes,
        secs
    );

    let bytes = formatted.into_bytes();
    unsafe {
        let layout = Layout::from_size_align(bytes.len() + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
        *out.add(bytes.len()) = 0;
        out
    }
}

/// Check if a year is a leap year.
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get the number of days in each month.
fn get_month_days(_year: i64, leap: bool) -> [i64; 12] {
    if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let now = __time_now();
        assert!(now > 0);
        // Should be after 2020-01-01
        assert!(now > 1577836800);
    }

    #[test]
    fn test_timestamp() {
        let ts = __time_timestamp();
        assert!(ts > 0);
        // Should be after 2020-01-01 in milliseconds
        assert!(ts > 1577836800000);
    }

    #[test]
    fn test_format() {
        // 2024-01-01 00:00:00 UTC
        let timestamp = 1704067200;
        let formatted = __time_format(timestamp);
        assert!(!formatted.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(formatted) }
            .to_str()
            .unwrap();
        assert_eq!(s, "2024-01-01 00:00:00");
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }
}
