use crate::utils::validate_basic_auth;
use clap::{Parser, ValueHint, value_parser};
use std::fs;
use std::path::PathBuf;
use url::Url;

/// Default values used throughout the project.
pub mod defaults {
    /// Maximum number of concurrent network requests.
    pub const SEMAPHORE: u8 = 4;

    /// The default timeout for network requests, in seconds.
    pub const TIMEOUT: u64 = 10;

    /// The default user agent header value used for network requests.
    pub const USER_AGENT: &str = concat!(
        "Mozilla/5.0 (compatible; Siteprobe/",
        env!("CARGO_PKG_VERSION"),
        ")"
    );

    /// The maximum number of slow documents to show
    pub const SLOW_NUM: i32 = 100;
}

fn validate_output_dir_str(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && path.is_dir() {
        println!(
            "\n⚠️ The output directory '{}' already exists. Existing documents will be overwritten.\n",
            path.display()
        );
        Ok(path)
    } else if path.exists() && !path.is_dir() {
        Err(format!(
            "❌ The output path '{}' is not a directory or is not writable.",
            path.display()
        ))
    } else {
        fs::create_dir_all(&path).map_err(|e| format!("Failed to create directory: {}", e))?;
        Ok(path)
    }
}

#[derive(Debug)]
enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
}

fn parse_rate_limit(value: &str) -> Result<u32, String> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err("Rate limit must be in the format 'requests/time[unit]'".to_string());
    }

    let requests: u32 = parts[0].parse().map_err(|_| "Invalid request count")?;
    let time_str = parts[1];

    if time_str.is_empty() {
        return Err("Time value cannot be empty".to_string());
    }

    let unit = match time_str.chars().last().unwrap() {
        's' => TimeUnit::Seconds,
        'm' => TimeUnit::Minutes,
        'h' => TimeUnit::Hours,
        _ => return Err("Time unit must be 's', 'm', or 'h'.".to_string()),
    };

    let time_value: u64 = time_str[..time_str.len() - 1]
        .parse()
        .map_err(|_| "Invalid time value")?;

    if time_value == 0 {
        return Err("Time value must be greater than 0".to_string());
    }

    let duration_secs = match unit {
        TimeUnit::Seconds => time_value,
        TimeUnit::Minutes => time_value * 60,
        TimeUnit::Hours => time_value * 3600,
    };

    let requests_per_minute = ((requests as f64) * 60.0 / (duration_secs as f64)).floor() as u32;

    // Calculated Requests per minute must be at least 1
    if requests_per_minute == 0 {
        return Err("Ensure the calculated rate is ≥ 1 per minute.".to_string());
    }

    Ok(requests_per_minute)
}

fn parse_slow_threshold(value: &str) -> Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("'{}' is not a valid number.", value))?;
    if parsed < 0.0 {
        return Err(format!(
            "Value '{}' must be greater than or equal to 0.0.",
            value
        ));
    }
    Ok(parsed)
}

#[derive(Debug, Parser)]
#[command(term_width = 80)]
pub struct Cli {
    #[arg(
        help = "The URL of the sitemap to be fetched and processed.",
        value_hint = ValueHint::Url,
        value_parser = value_parser!(Url)
    )]
    pub sitemap_url: Url,

    #[arg(
        long,
        help = "Basic authentication credentials in the format `username:password`",
        value_parser = validate_basic_auth,
    )]
    pub basic_auth: Option<String>,

    #[arg(
        short = 'c',
        long,
        help = "Maximum number of concurrent requests allowed",
        default_value_t = defaults::SEMAPHORE as u8,
        value_parser = clap::value_parser!(u8).range(1..=100)
    )]
    pub concurrency_limit: u8,

    #[arg(
        short = 'l',
        long,
        help = "The rate limit for all requests in the format 'requests/time[unit]', where unit can be seconds (`s`), minutes (`m`), or hours (`h`). E.g. '-l 300/5m' for 300 requests per 5 minutes, or '-l 100/1h' for 100 requests per hour.",
        value_parser = parse_rate_limit
    )]
    pub rate_limit: Option<u32>, // Returns requests per 1 minute

    #[arg(
        short = 'o',
        long,
        help = "Directory where all downloaded documents will be saved",
        value_hint = ValueHint::DirPath,
        value_parser = validate_output_dir_str
    )]
    pub output_dir: Option<PathBuf>,

    #[arg(
        short = 'a',
        long,
        help = "Append a random timestamp to each URL to bypass caching mechanisms",
        default_value = "false"
    )]
    pub append_timestamp: bool,

    #[arg(
        short = 'r',
        long,
        help = "File path for storing the generated `report.csv`",
        value_hint = ValueHint::FilePath,
        value_parser = clap::value_parser!(PathBuf)
    )]
    pub report_path: Option<PathBuf>,

    #[arg(
        short = 'j',
        long,
        help = "File path for storing the generated `report.json`",
        value_hint = ValueHint::FilePath,
        value_parser = clap::value_parser!(PathBuf)
    )]
    pub report_path_json: Option<PathBuf>,

    #[arg(
        short = 't',
        long,
        help = "Default timeout (in seconds) for each request",
        default_value_t = defaults::TIMEOUT as u8,
        value_parser = clap::value_parser!(u8).range(1..=60)
    )]
    pub request_timeout: u8,

    #[arg(
        long,
        help = "Custom User-Agent header to be used in requests",
        default_value_t = defaults::USER_AGENT.to_string(),
    )]
    pub user_agent: String,

    #[arg(
        long,
        help = "Limit the number of slow documents displayed in the report.",
        default_value_t = defaults::SLOW_NUM as u32,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub slow_num: u32,

    #[arg(
        short = 's',
        long,
        help = "Show slow responses. The value is the threshold (in seconds) for considering a document as 'slow'. E.g. '-s 3' for 3 seconds or '-s 0.05' for 50ms.",
        value_parser = parse_slow_threshold,
    )]
    pub slow_threshold: Option<f64>,

    #[arg(
        short = 'f',
        long,
        help = "Controls automatic redirects. When enabled, the client will follow HTTP redirects (up to 10 by default). Note that for security, Basic Authentication credentials are intentionally not forwarded during redirects to prevent unintended credential exposure."
    )]
    pub follow_redirects: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
