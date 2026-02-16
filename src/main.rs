mod metrics;
mod network;
mod options;
mod report;
mod sitemap;
mod storage;
mod utils;

use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;

use crate::sitemap::{fetch_and_generate_report, get_sitemap_urls};
use clap::Parser;
use console::style;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    // Parse terminal arguments.
    let mut options = options::Cli::parse();

    // Load config file and apply values (CLI args take priority).
    let config = options::ConfigFile::load(options.config.as_ref()).unwrap_or_else(|e| {
        eprintln!("{} {}", style("[ERROR]").red(), e);
        std::process::exit(1);
    });
    options.apply_config(&config);

    // Build the HTTP client.
    let client = Arc::new(network::build_client(&options)?);
    let start_time = Instant::now();

    // Fetch all URLs from the sitemap.
    let urls = get_sitemap_urls(options.sitemap_url.as_str(), &client, options.json)
        .await
        .unwrap_or_else(|e| {
            eprintln!("{} {}", style("[ERROR]").red(), e);
            std::process::exit(1);
        });

    // Fetch URLs concurrently and generate a report.
    let report = fetch_and_generate_report(urls, &client, &options, &start_time).await?;

    if options.json {
        // Print clean JSON to stdout for piping.
        println!("{}", report.to_json_string(&options)?);
    } else {
        // Display the report.
        report.show_text_report(&options);
    }

    // Optionally, write the report to CSV file.
    if let Some(path) = options.report_path.as_ref() {
        report.write_csv_report(path, options.json)?;
    }

    // Optionally, write the report to JSON file.
    if let Some(path) = options.report_path_json.as_ref() {
        report.write_json_report(&options, path)?;
    }

    // Optionally, write the report to HTML file.
    if let Some(path) = options.report_path_html.as_ref() {
        report.write_html_report(&options, path)?;
    }

    Ok(report.exit_code(options.slow_threshold))
}
