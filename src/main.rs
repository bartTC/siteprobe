use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{CommandFactory, FromArgMatches};
use console::style;
use tokio::time::Instant;

use siteprobe::network;
use siteprobe::options::{Cli, ConfigFile};
use siteprobe::sitemap::{fetch_and_generate_report, get_sitemap_urls};

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    // Parse terminal arguments. Keep the raw matches around so config merging
    // can tell which options were explicitly set on the command line.
    let matches = Cli::command().get_matches();
    let mut options = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // Load config file and apply values (CLI args take priority).
    let config = ConfigFile::load(options.config.as_deref()).unwrap_or_else(|e| {
        eprintln!("{} {}", style("[ERROR]").red(), e);
        std::process::exit(1);
    });
    options.apply_config(&config, &matches);

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
