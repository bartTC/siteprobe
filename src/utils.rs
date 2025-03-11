use rand::Rng;
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
/// let message = "👩‍🚀🌌"; // Astronaut emoji followed by the Milky Way emoji
/// let truncated = truncate_message(message, 1);
/// assert_eq!(truncated, "👩‍🚀…");
///
/// let short_message = "Hi";
/// let truncated = truncate_message(short_message, 2);
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
/// ```rust
/// use your_crate::utils::generate_random_number;
///
/// let random_number = generate_random_number(4);
/// println!("Generated random number: {}", random_number);
/// ```
pub fn generate_random_number(length: u32) -> u64 {
    assert!(length > 0, "length must be greater than 0");
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

#[cfg(test)]
mod tests {
    use super::*;
    /// --------------------------------------------------------------------------------------------
    /// truncate_message Tests
    #[test]
    fn test_truncate_message_no_truncation_needed() {
        assert_eq!(truncate_message("Short", 10), "Short");
    }

    #[test]
    fn test_truncate_message_with_truncation() {
        let result = truncate_message("This is a long message", 10);
        assert_eq!(result, "This is a…");
    }

    #[test]
    fn test_truncate_message_exact_width() {
        let result = truncate_message("Exact size", 10);
        assert_eq!(result, "Exact size");
    }

    #[test]
    fn test_truncate_message_empty_string() {
        let message = "";
        let result = truncate_message(message, 5);
        assert_eq!(result, "");
    }

    #[test]
    #[should_panic(expected = "max_width must be at least 2 to accommodate the ellipsis")]
    fn test_truncate_message_max_width_one() {
        truncate_message("Something", 1);
    }

    #[test]
    fn test_truncate_message_unicode_handling() {
        let message = "こんにちは世界"; // "Hello World" in Japanese
        let result = truncate_message(message, 6);
        assert_eq!(result, "こんにちは…");
    }

    #[test]
    fn test_truncate_message_emoji_handling() {
        let message = "😽‍🧝🏼‍♂️🏫🤣👨";
        let result = truncate_message(message, 3);
        assert_eq!(result, "😽‍🧝🏼‍♂️🏫…");
    }

    /// --------------------------------------------------------------------------------------------
    /// validate_basic_auth Tests
    #[test]
    fn test_valid_basic_auth() {
        assert!(validate_basic_auth("user:pass").is_ok());
        assert!(validate_basic_auth("user@domain.com:password123").is_ok());
        assert!(validate_basic_auth("user:pass:with:colon").is_ok());
    }

    #[test]
    fn test_invalid_basic_auth() {
        assert!(validate_basic_auth("invalid").is_err());
        assert!(validate_basic_auth("").is_err());
        assert!(validate_basic_auth(":").is_err());
        assert!(validate_basic_auth("user:").is_err());
        assert!(validate_basic_auth(":pass").is_err());
    }

    /// --------------------------------------------------------------------------------------------
    /// generate_random_number Tests
    use std::collections::HashSet;

    #[test]
    fn test_generate_random_number_valid_length() {
        let length = 5;
        let number = generate_random_number(length);
        let number_str = number.to_string();

        assert_eq!(
            number_str.len(),
            length as usize,
            "Generated number does not match the specified length"
        );
    }

    #[test]
    fn test_generate_random_number_min_length() {
        let length = 1;
        let number = generate_random_number(length);
        assert!(
            (1..=9).contains(&number),
            "Generated number is outside the range for length 1"
        );
    }

    #[test]
    fn test_generate_random_number_randomness() {
        let length = 4;

        // Generate multiple numbers and collect them into a HashSet to check for uniqueness
        let mut numbers = HashSet::new();
        for _ in 0..1000 {
            let number = generate_random_number(length);
            numbers.insert(number);
        }

        assert!(
            numbers.len() > 900,
            "Generated numbers are not sufficiently random (too many duplicates found)"
        );
    }

    #[test]
    #[should_panic(expected = "length must be greater than 0")]
    fn test_generate_random_number_zero_length() {
        // Passing a length of zero should panic because `10u64.pow(length - 1)` will underflow
        generate_random_number(0);
    }
}
