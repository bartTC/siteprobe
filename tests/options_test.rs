use siteprobe::options::parse_rate_limit;

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
