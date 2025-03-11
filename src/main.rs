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
    let options = options::Cli::parse();

    // Build the HTTP client.
    let client = Arc::new(network::build_client(&options)?);
    let start_time = Instant::now();

    // Fetch all URLs from the sitemap.
    let urls = get_sitemap_urls(options.sitemap_url.as_str(), &client)
        .await
        .map_err(|e| {
            eprintln!("{} Unable to fetch sitemap: {}", style("[ERROR]").red(), e);
            e
        })?;

    // Fetch URLs concurrently and generate a report.
    let report = fetch_and_generate_report(urls, client, &options, start_time).await?;

    // Display the report.
    report.show_text_report(&options);

    // Optionally, write report to CSV file.
    if let Some(path) = options.report_path.as_ref() {
        report.write_csv_report(path)?;
    }

    Ok(ExitCode::SUCCESS)
}
