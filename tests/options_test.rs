use siteprobe::options::{UrlSource, parse_rate_limit, parse_target_url, parse_url_source};
use std::io::Write;
use std::process::Command;
use url::Url;

#[test]
fn test_parse_rate_limit_valid_inputs() {
    // Valid input: 60 requests per 1 second
    let input = "60/1s";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(3600)); // 60 requests * 60 seconds per minute

    // Valid input: 30 requests per 2 minutes
    let input = "30/2m";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(15)); // 30 requests / 2 minutes

    // Valid input: 360 requests per 1 hour
    let input = "360/1h";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(6)); // 360 requests / 60 minutes in an hour
}

#[test]
fn test_parse_rate_limit_invalid_formats() {
    // Missing slash
    let input = "100m";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Rate limit must be in the format 'requests/time[unit]'"
    );

    // Extra slash
    let input = "50/2/m";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Rate limit must be in the format 'requests/time[unit]'"
    );

    // Invalid format in requests
    let input = "xyz/1s";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Invalid request count");

    // Invalid format in time value
    let input = "100/xyzs";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Invalid time value");

    // Empty time value
    let input = "100/s";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Invalid time value");
}

#[test]
fn test_parse_rate_limit_invalid_units() {
    // Invalid time unit
    let input = "100/1x";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Time unit must be 's', 'm', or 'h'.");

    // Missing time unit
    let input = "100/1";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Time unit must be 's', 'm', or 'h'.");
}

#[test]
fn test_parse_rate_limit_invalid_time_value() {
    // Time value is zero
    let input = "100/0s";
    let result = parse_rate_limit(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Time value must be greater than 0");
}

#[test]
fn test_parse_rate_limit_edge_cases() {
    // Minimal valid input (1 request per 1 second)
    let input = "1/1s";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(60)); // 1 request per second = 60 requests per minute

    // Very high number of requests per hour
    let input = "1000000000/1h";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(16666666));

    // 1 request per 1 minute
    let input = "1/1m";
    let result = parse_rate_limit(input);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_parse_rate_limit_at_least_one_per_minute() {
    // Calculated rate must be at least 1 per minute
    let input = "1/2m";
    let result = parse_rate_limit(input);
    assert_eq!(
        result.err().unwrap(),
        "Ensure the calculated rate is ≥ 1 per minute."
    );

    let input = "59/1h";
    let result = parse_rate_limit(input);
    assert_eq!(
        result.err().unwrap(),
        "Ensure the calculated rate is ≥ 1 per minute."
    );

    let input = "1/1h";
    let result = parse_rate_limit(input);
    assert_eq!(
        result.err().unwrap(),
        "Ensure the calculated rate is ≥ 1 per minute."
    );

    let input = "1/120s";
    let result = parse_rate_limit(input);
    assert_eq!(
        result.err().unwrap(),
        "Ensure the calculated rate is ≥ 1 per minute."
    );
}

#[test]
fn test_parse_rate_limit_empty_time() {
    let result = parse_rate_limit("100/");
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Time value cannot be empty");
}

#[test]
fn test_slow_threshold_invalid_via_cli() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "-s",
            "notanumber",
        ])
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a valid number"),
        "Expected 'not a valid number' error, got: {}",
        stderr
    );
}

#[test]
fn test_slow_threshold_negative_via_cli() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "-s",
            "-1.0",
        ])
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("greater than or equal to 0.0") || stderr.contains("unexpected argument"),
        "Expected validation error, got: {}",
        stderr
    );
}

// ===========================================================================================
// parse_url_source Tests
// ===========================================================================================

fn temp_file() -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .prefix("siteprobe_options_test_")
        .suffix(".txt")
        .tempfile()
        .expect("Failed to create temp file");
    file.write_all(b"https://example.com/\n")
        .expect("Failed to write temp file");
    file
}

#[test]
fn test_parse_url_source_dash_is_stdin() {
    assert_eq!(parse_url_source("-"), Ok(UrlSource::Stdin));
}

#[test]
fn test_parse_url_source_http_urls_are_remote() {
    assert_eq!(
        parse_url_source("https://example.com/sitemap.xml"),
        Ok(UrlSource::Remote(
            Url::parse("https://example.com/sitemap.xml").unwrap()
        ))
    );
    assert_eq!(
        parse_url_source("http://localhost:8000/urls.txt"),
        Ok(UrlSource::Remote(
            Url::parse("http://localhost:8000/urls.txt").unwrap()
        ))
    );
}

#[test]
fn test_parse_url_source_existing_file() {
    let file = temp_file();
    let result = parse_url_source(file.path().to_str().unwrap());
    assert_eq!(result, Ok(UrlSource::File(file.path().to_path_buf())));
}

#[test]
fn test_parse_url_source_file_url() {
    let file = temp_file();
    let file_url = Url::from_file_path(file.path()).unwrap();
    let result = parse_url_source(file_url.as_str());
    assert_eq!(result, Ok(UrlSource::File(file.path().to_path_buf())));
}

#[test]
fn test_parse_url_source_missing_file_is_an_error() {
    let result = parse_url_source("/definitely/not/here/urls.txt");
    let err = result.expect_err("missing file should be rejected");
    assert!(err.contains("neither an http(s) URL"), "{}", err);
}

#[test]
fn test_parse_url_source_directory_is_an_error() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let result = parse_url_source(dir.path().to_str().unwrap());
    assert!(result.is_err(), "a directory is not a valid source");
}

#[test]
fn test_parse_url_source_scheme_less_url_is_an_error() {
    // Without a scheme this is treated as a path, which does not exist.
    let result = parse_url_source("example.com/sitemap.xml");
    assert!(result.is_err());
}

// ===========================================================================================
// parse_target_url Tests
// ===========================================================================================

#[test]
fn test_parse_target_url_valid() {
    assert_eq!(
        parse_target_url("https://example.com/about"),
        Ok(Url::parse("https://example.com/about").unwrap())
    );
    assert_eq!(
        parse_target_url("http://example.com"),
        Ok(Url::parse("http://example.com/").unwrap())
    );
}

#[test]
fn test_parse_target_url_rejects_other_schemes() {
    let err = parse_target_url("ftp://example.com/file").expect_err("ftp should be rejected");
    assert!(err.contains("http or https"), "{}", err);
}

#[test]
fn test_parse_target_url_rejects_relative_urls() {
    let err = parse_target_url("/about").expect_err("relative URL should be rejected");
    assert!(err.contains("not a valid URL"), "{}", err);
}
