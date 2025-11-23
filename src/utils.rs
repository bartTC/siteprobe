use rand::Rng;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

/// Truncates a given string to a specified maximum width, appending an ellipsis (`…`)
/// if the string exceeds the specified width. Handles Unicode grapheme clusters properly.
///
/// # Parameters
/// - `message`: The input string to be truncated.
/// - `max_width`: The maximum allowed display width of the string, including the space for the ellipsis.
///   Must be at least 2 to accommodate the ellipsis.
///
/// # Returns
/// A new `String` that is either the original string (if its grapheme count is less than or equal to `max_width`)
/// or a truncated version with an ellipsis appended.
///
/// # Panics
/// Panics if `max_width` is less than 2.
///
/// # Examples
/// ```rust
/// use siteprobe::utils::truncate_message;
///
/// let message = "Hello World";
/// let truncated = truncate_message(message, 6);
/// assert_eq!(truncated, "Hello…");
///
/// let short_message = "Hi";
/// let truncated = truncate_message(short_message, 5);
/// assert_eq!(truncated, "Hi");
/// ```
pub fn truncate_message(message: &str, max_width: usize) -> String {
    // Ensure max_width is at least 2
    assert!(
        max_width >= 2,
        "max_width must be at least 2 to accommodate the ellipsis"
    );

    let ellipsis = "…";

    // Use `unicode-segmentation` to split the message into grapheme clusters
    let graphemes: Vec<&str> = message.graphemes(true).collect();

    if graphemes.len() > max_width {
        // Truncate to max_width - 1 to leave space for ellipsis
        let truncated: String = graphemes[..max_width - 1].concat();
        format!("{}{}", truncated, ellipsis)
    } else {
        message.to_string()
    }
}

/// Generates a random number with the specified number of digits.
///
/// # Arguments
///
/// * `length` - The number of digits for the random number. Must be greater than 0.
///
/// # Returns
///
/// A `u64` random number with exactly `length` digits.
/// If `length` is 1, the generated number will be in the range [0, 9].
///
/// # Panics
///
/// This function will panic if `length` is 0, as it causes an invalid range for number generation.
///
/// # Examples
///
/// ```rust,no_run
/// use siteprobe::utils::generate_random_number;
///
/// let random_number = generate_random_number(4);
/// println!("Generated random number: {}", random_number);
/// ```
pub fn generate_random_number(length: u32) -> u64 {
    assert!(length > 0, "length must be greater than 0");
    assert!(length <= 19, "length must be at most 19 to fit in u64");
    let range = 10u64.pow(length - 1)..10u64.pow(length);
    rand::rng().random_range(range)
}

/// Validates a basic HTTP authentication string in the format `username:password`.
///
/// # Arguments
///
/// * `val` - A string slice representing the credentials to validate.
///
/// # Returns
///
/// * `Ok(String)` - If the input string is in the correct `username:password` format.
/// * `Err(String)` - If the input string is not valid, with an error message describing the issue.
///
/// # Errors
///
/// This function returns an error if:
/// - The input does not contain exactly one colon (`:`) separating the username and password.
/// - The input lacks either the username or the password.
///
/// # Example
///
/// ```rust
/// use siteprobe::utils::validate_basic_auth;
///
/// let valid = validate_basic_auth("user:pass");
/// assert!(valid.is_ok());
///
/// let invalid = validate_basic_auth("invalid_format");
/// assert!(invalid.is_err());
/// ```
pub fn validate_basic_auth(val: &str) -> Result<String, String> {
    if val.contains(':') {
        let parts: Vec<&str> = val.splitn(2, ':').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Ok(val.to_string())
        } else {
            Err(String::from(
                "Invalid format: must be `username:password` with non-empty values",
            ))
        }
    } else {
        Err(String::from("Invalid format: must be `username:password`"))
    }
}

pub fn kb(bytes: usize) -> String {
    let kilobytes = bytes as f64 / 1024.0;
    format!("{kilobytes:.2}kb")
}

pub fn percent(percent: f64) -> String {
    format!("{percent:.0}%")
}

pub fn ms(duration: Duration) -> String {
    let milliseconds = duration.as_millis() as f64;
    format!("{milliseconds:.2}ms")
}
