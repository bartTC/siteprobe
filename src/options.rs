use crate::utils::validate_basic_auth;
use clap::{value_parser, Parser, ValueHint};
use serde::Deserialize;
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
    pub const SLOW_NUM: u32 = 100;

    /// The default number of retries for failed requests.
    pub const RETRIES: u8 = 0;
}

/// Expands shell-style tilde (`~`) in paths to the user's home directory.
pub fn expand_path(s: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(shellexpand::tilde(s).into_owned()))
}

fn validate_output_dir_str(s: &str) -> Result<PathBuf, String> {
    let path = expand_path(s)?;
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

pub fn parse_rate_limit(value: &str) -> Result<u32, String> {
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
#[command(term_width = 80, version, after_help = "\
EXIT CODES:\n\
    0  All URLs returned 2xx (success)\n\
    1  One or more URLs returned 4xx/5xx or failed\n\
    2  One or more URLs exceeded the slow threshold (--slow-threshold)"
)]
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
        value_parser = expand_path
    )]
    pub report_path: Option<PathBuf>,

    #[arg(
        short = 'j',
        long,
        help = "File path for storing the generated `report.json`",
        value_hint = ValueHint::FilePath,
        value_parser = expand_path
    )]
    pub report_path_json: Option<PathBuf>,

    #[arg(
        long,
        help = "File path for storing the generated `report.html`",
        value_hint = ValueHint::FilePath,
        value_parser = expand_path
    )]
    pub report_path_html: Option<PathBuf>,

    #[arg(
        short = 't',
        long,
        help = "Default timeout (in seconds) for each request",
        default_value_t = defaults::TIMEOUT,
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub request_timeout: u64,

    #[arg(
        long,
        help = "Custom User-Agent header to be used in requests",
        default_value_t = defaults::USER_AGENT.to_string(),
    )]
    pub user_agent: String,

    #[arg(
        long,
        help = "Limit the number of slow documents displayed in the report.",
        default_value_t = defaults::SLOW_NUM,
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

    #[arg(
        long,
        help = "Number of retries for failed requests (network errors or 5xx responses)",
        default_value_t = defaults::RETRIES,
        value_parser = clap::value_parser!(u8).range(0..=10)
    )]
    pub retries: u8,

    #[arg(
        long,
        help = "Output the JSON report to stdout instead of the normal table output. Suppresses all other console output for clean piping.",
        default_value = "false"
    )]
    pub json: bool,

    #[arg(
        long,
        help = "Path to a TOML config file. Defaults to `.siteprobe.toml` in the current directory.",
        value_hint = ValueHint::FilePath,
        value_parser = expand_path
    )]
    pub config: Option<PathBuf>,
}

/// Represents settings loaded from a `.siteprobe.toml` config file.
/// All fields are optional; only those present in the file will override defaults.
#[derive(Debug, Default, Deserialize)]
pub struct ConfigFile {
    pub user_agent: Option<String>,
    pub concurrency_limit: Option<u8>,
    pub rate_limit: Option<String>,
    pub request_timeout: Option<u64>,
    pub slow_threshold: Option<f64>,
    pub slow_num: Option<u32>,
    pub basic_auth: Option<String>,
    pub follow_redirects: Option<bool>,
    pub append_timestamp: Option<bool>,
    pub retries: Option<u8>,
    pub report_path: Option<String>,
    pub report_path_json: Option<String>,
    pub report_path_html: Option<String>,
}

impl ConfigFile {
    /// Load a config file from the given path, or return a default (empty) config.
    pub fn load(path: Option<&PathBuf>) -> Result<Self, String> {
        let config_path = match path {
            Some(p) => {
                if !p.exists() {
                    return Err(format!(
                        "Config file '{}' not found.",
                        p.display()
                    ));
                }
                p.clone()
            }
            None => {
                let default_path = PathBuf::from(".siteprobe.toml");
                if !default_path.exists() {
                    return Ok(Self::default());
                }
                default_path
            }
        };

        let contents = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file '{}': {}", config_path.display(), e))?;
        let config: ConfigFile = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", config_path.display(), e))?;
        Ok(config)
    }
}

/// Returns true if the user explicitly provided the given CLI argument
/// by scanning the raw command-line arguments for the long or short flag.
fn arg_provided(name: &str) -> bool {
    let short_map: &[(&str, &str)] = &[
        ("concurrency_limit", "-c"),
        ("rate_limit", "-l"),
        ("output_dir", "-o"),
        ("append_timestamp", "-a"),
        ("report_path", "-r"),
        ("report_path_json", "-j"),
        ("request_timeout", "-t"),
        ("slow_threshold", "-s"),
        ("follow_redirects", "-f"),
    ];
    let long_with_dash = format!("--{}", name.replace('_', "-"));
    let args: Vec<String> = std::env::args().collect();
    for arg in &args {
        if arg == &long_with_dash || arg.starts_with(&format!("{}=", long_with_dash)) {
            return true;
        }
        if let Some((_, short)) = short_map.iter().find(|(n, _)| *n == name) {
            if arg == *short {
                return true;
            }
        }
    }
    false
}

impl Cli {
    /// Merge config file values into the CLI options.
    /// CLI arguments take priority over config file values.
    pub fn apply_config(&mut self, config: &ConfigFile) {
        if let Some(ref v) = config.user_agent {
            if !arg_provided("user_agent") {
                self.user_agent = v.clone();
            }
        }
        if let Some(v) = config.concurrency_limit {
            if !arg_provided("concurrency_limit") {
                self.concurrency_limit = v;
            }
        }
        if let Some(ref v) = config.rate_limit {
            if !arg_provided("rate_limit") {
                match parse_rate_limit(v) {
                    Ok(rpm) => self.rate_limit = Some(rpm),
                    Err(e) => eprintln!("Warning: invalid rate_limit in config file: {}", e),
                }
            }
        }
        if let Some(v) = config.request_timeout {
            if !arg_provided("request_timeout") {
                self.request_timeout = v;
            }
        }
        if let Some(v) = config.slow_threshold {
            if !arg_provided("slow_threshold") {
                self.slow_threshold = Some(v);
            }
        }
        if let Some(v) = config.slow_num {
            if !arg_provided("slow_num") {
                self.slow_num = v;
            }
        }
        if let Some(ref v) = config.basic_auth {
            if !arg_provided("basic_auth") {
                self.basic_auth = Some(v.clone());
            }
        }
        if let Some(v) = config.follow_redirects {
            if !arg_provided("follow_redirects") {
                self.follow_redirects = v;
            }
        }
        if let Some(v) = config.append_timestamp {
            if !arg_provided("append_timestamp") {
                self.append_timestamp = v;
            }
        }
        if let Some(v) = config.retries {
            if !arg_provided("retries") {
                self.retries = v;
            }
        }
        if let Some(ref v) = config.report_path {
            if !arg_provided("report_path") {
                self.report_path = expand_path(v).ok();
            }
        }
        if let Some(ref v) = config.report_path_json {
            if !arg_provided("report_path_json") {
                self.report_path_json = expand_path(v).ok();
            }
        }
        if let Some(ref v) = config.report_path_html {
            if !arg_provided("report_path_html") {
                self.report_path_html = expand_path(v).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_tilde() {
        let home = std::env::var("HOME").expect("HOME not set");
        let result = expand_path("~/test/path").unwrap();
        assert_eq!(result, PathBuf::from(format!("{}/test/path", home)));
    }

    #[test]
    fn test_expand_path_tilde_only() {
        let home = std::env::var("HOME").expect("HOME not set");
        let result = expand_path("~").unwrap();
        assert_eq!(result, PathBuf::from(home));
    }

    #[test]
    fn test_expand_path_no_tilde() {
        let result = expand_path("/absolute/path/to/file").unwrap();
        assert_eq!(result, PathBuf::from("/absolute/path/to/file"));
    }

    #[test]
    fn test_expand_path_relative() {
        let result = expand_path("relative/path").unwrap();
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_path_tilde_in_middle_not_expanded() {
        // Tilde in the middle of a path should NOT be expanded
        let result = expand_path("/path/~/to/file").unwrap();
        assert_eq!(result, PathBuf::from("/path/~/to/file"));
    }
}
