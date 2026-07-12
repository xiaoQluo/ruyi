/**
 * C FFI implementations backing `stdlib/json.ry`.
 *
 * Provides JSON parsing and stringification via C ABI for use by Ruyi standard library.
 * This is a basic implementation that handles JSON objects, arrays, strings, numbers,
 * booleans, and null values.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use std::ffi::{CStr, CString};

/// Parse a JSON string and return a Ruyi value.
///
/// Returns a pointer to the parsed value, or null on error.
///
/// # Safety
/// `json_str` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn __json_parse(json_str: *const i8) -> *mut i8 {
    if json_str.is_null() {
        return std::ptr::null_mut();
    }

    let json = match CStr::from_ptr(json_str).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Simple JSON parser implementation
    // This is a basic implementation - a full implementation would use a proper parser
    match parse_json_value(json) {
        Ok((_, value)) => {
            let c_string = CString::new(value).unwrap();
            c_string.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Stringify a Ruyi value to JSON.
///
/// Returns a pointer to the JSON string, or null on error.
///
/// # Safety
/// `value` must be a valid null-terminated C string representing a Ruyi value.
#[no_mangle]
pub unsafe extern "C" fn __json_stringify(value: *const i8) -> *mut i8 {
    if value.is_null() {
        return std::ptr::null_mut();
    }

    let val_str = match CStr::from_ptr(value).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Simple JSON stringifier implementation
    // This is a basic implementation - a full implementation would handle more cases
    match stringify_json_value(val_str) {
        Ok(json) => {
            let c_string = CString::new(json).unwrap();
            c_string.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Parse a JSON value from string (simplified implementation).
fn parse_json_value(input: &str) -> Result<(&str, String), String> {
    let input = input.trim();

    if input.is_empty() {
        return Err("Empty input".to_string());
    }

    // Parse null
    if input.starts_with("null") {
        return Ok((&input[4..], "null".to_string()));
    }

    // Parse boolean
    if input.starts_with("true") {
        return Ok((&input[4..], "true".to_string()));
    }
    if input.starts_with("false") {
        return Ok((&input[5..], "false".to_string()));
    }

    // Parse string
    if input.starts_with('"') {
        let end = find_string_end(&input[1..])?;
        let string_content = &input[1..end];
        let remaining = &input[end + 1..];
        return Ok((remaining, format!("\"{}\"", string_content)));
    }

    // Parse number
    if input.starts_with('-') || input.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        let end = find_number_end(input)?;
        let number = &input[..end];
        return Ok((&input[end..], number.to_string()));
    }

    // Parse array
    if input.starts_with('[') {
        return parse_json_array(&input[1..]);
    }

    // Parse object
    if input.starts_with('{') {
        return parse_json_object(&input[1..]);
    }

    Err(format!(
        "Unexpected character: {}",
        input.chars().next().unwrap_or(' ')
    ))
}

/// Find the end of a JSON string.
fn find_string_end(input: &str) -> Result<usize, String> {
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2; // Skip escaped character
        } else if chars[i] == '"' {
            return Ok(i + 1); // Include the closing quote
        } else {
            i += 1;
        }
    }

    Err("Unterminated string".to_string())
}

/// Find the end of a JSON number.
fn find_number_end(input: &str) -> Result<usize, String> {
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    // Optional minus sign
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }

    // Integer part
    if i < chars.len() && chars[i] == '0' {
        i += 1;
    } else if i < chars.len() && chars[i].is_ascii_digit() {
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return Err("Invalid number".to_string());
    }

    // Fractional part
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        if i < chars.len() && chars[i].is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            return Err("Invalid number: expected digit after decimal point".to_string());
        }
    }

    // Exponent part
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        if i < chars.len() && chars[i].is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            return Err("Invalid number: expected digit in exponent".to_string());
        }
    }

    Ok(i)
}

/// Parse a JSON array.
fn parse_json_array(input: &str) -> Result<(&str, String), String> {
    let mut result = String::from("[");
    let mut remaining = input.trim();
    let mut first = true;

    while !remaining.is_empty() {
        remaining = remaining.trim();

        if remaining.starts_with(']') {
            result.push(']');
            return Ok((&remaining[1..], result));
        }

        if !first {
            if !remaining.starts_with(',') {
                return Err("Expected comma in array".to_string());
            }
            remaining = &remaining[1..];
        }

        let (new_remaining, value) = parse_json_value(remaining)?;
        remaining = new_remaining.trim();

        if !first {
            result.push(',');
        }
        result.push_str(&value);
        first = false;
    }

    Err("Unterminated array".to_string())
}

/// Parse a JSON object.
fn parse_json_object(input: &str) -> Result<(&str, String), String> {
    let mut result = String::from("{");
    let mut remaining = input.trim();
    let mut first = true;

    while !remaining.is_empty() {
        remaining = remaining.trim();

        if remaining.starts_with('}') {
            result.push('}');
            return Ok((&remaining[1..], result));
        }

        if !first {
            if !remaining.starts_with(',') {
                return Err("Expected comma in object".to_string());
            }
            remaining = &remaining[1..].trim();
        }

        // Parse key
        if !remaining.starts_with('"') {
            return Err("Expected string key in object".to_string());
        }
        let (new_remaining, key) = parse_json_value(remaining)?;
        remaining = new_remaining.trim();

        // Parse colon
        if !remaining.starts_with(':') {
            return Err("Expected colon in object".to_string());
        }
        remaining = &remaining[1..].trim();

        // Parse value
        let (new_remaining, value) = parse_json_value(remaining)?;
        remaining = new_remaining.trim();

        if !first {
            result.push(',');
        }
        result.push_str(&key);
        result.push(':');
        result.push_str(&value);
        first = false;
    }

    Err("Unterminated object".to_string())
}

/// Stringify a JSON value (simplified implementation).
fn stringify_json_value(value: &str) -> Result<String, String> {
    let value = value.trim();

    // Already a valid JSON value
    if value == "null" || value == "true" || value == "false" {
        return Ok(value.to_string());
    }

    // Number
    if value.starts_with('-') || value.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return Ok(value.to_string());
    }

    // String
    if value.starts_with('"') && value.ends_with('"') {
        return Ok(value.to_string());
    }

    // Array
    if value.starts_with('[') && value.ends_with(']') {
        return Ok(value.to_string());
    }

    // Object
    if value.starts_with('{') && value.ends_with('}') {
        return Ok(value.to_string());
    }

    // Treat as string if not recognized
    Ok(format!("\"{}\"", value.replace('"', "\\\"")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let result = unsafe { __json_parse(b"null\0".as_ptr() as *const i8) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "null");
    }

    #[test]
    fn test_parse_boolean() {
        let result = unsafe { __json_parse(b"true\0".as_ptr() as *const i8) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "true");
    }

    #[test]
    fn test_parse_string() {
        let result = unsafe { __json_parse(b"\"hello\"\0".as_ptr() as *const i8) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "\"hello\"");
    }

    #[test]
    fn test_parse_number() {
        let result = unsafe { __json_parse(b"42\0".as_ptr() as *const i8) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn test_stringify() {
        let result = unsafe { __json_stringify(b"hello\0".as_ptr() as *const i8) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "\"hello\"");
    }
}
