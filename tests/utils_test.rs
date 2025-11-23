use siteprobe::utils::{generate_random_number, truncate_message, validate_basic_auth};
use std::collections::HashSet;

// ===========================================================================================
// truncate_message Tests
// ===========================================================================================

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

// ===========================================================================================
// validate_basic_auth Tests
// ===========================================================================================

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

// ===========================================================================================
// generate_random_number Tests
// ===========================================================================================

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
