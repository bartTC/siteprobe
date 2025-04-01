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
