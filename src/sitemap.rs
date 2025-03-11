use crate::network::{get_url_content, get_url_response};
use crate::options::Cli;
use crate::report::Report;
use crate::utils;
use console::style;
use futures::future::join_all;
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Client;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::Instant;

// region: Structs & Enums
#[derive(Debug, PartialEq)]
pub enum SitemapType {
    SitemapIndex,
    UrlSet,
    Unknown,
}

// Implement Display for SitemapType
impl fmt::Display for SitemapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
// endregion

// region: Functions
pub async fn get_sitemap_urls(
    sitemap_url: &str,
    client: &Client,
) -> Result<Vec<String>, Box<dyn Error>> {
    let content = match get_url_content(sitemap_url, client).await {
        Ok(content) => content,
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let sitemap_type = identify_sitemap_type(&content);
    println!("{} 🔎 Fetch {}...", style("[1/3]").dim(), sitemap_type);

    if sitemap_type == SitemapType::Unknown {
        eprintln!(
            "{} The sitemap does not contain any urls: {}",
            style("[ERROR]").red(),
            &sitemap_url
        );
    }

    // A sitemap.xml file might be an index file, linking to other sitemaps.
    // In that case, retrieve the urls from all those sitemaps.
    let mut urls = Vec::new();

    println!(
        "{} 🚚 Collect all URLs from sitemap...",
        style("[2/3]").dim()
    );
    if sitemap_type == SitemapType::SitemapIndex {
        let sitemap_urls = extract_sitemap_urls(&content);
        for sitemap_url in sitemap_urls {
            match get_url_content(&sitemap_url, client).await {
                Ok(content) => {
                    urls.extend(extract_sitemap_urls(&content));
                }
                Err(_) => {
                    eprintln!(
                        "{} The referenced sitemap is missing: {}",
                        style("[ERROR]").red(),
                        &sitemap_url
                    );
                }
            };
        }
    } else if sitemap_type == SitemapType::UrlSet {
        urls.extend(extract_sitemap_urls(&content));
    }

    Ok(urls)
}

pub fn identify_sitemap_type(xml: &str) -> SitemapType {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                return match e.name().as_ref() {
                    b"sitemapindex" => SitemapType::SitemapIndex,
                    b"urlset" => SitemapType::UrlSet,
                    _ => SitemapType::Unknown,
                };
            }
            Ok(Event::Eof) => break, // End of file
            Err(_) => return SitemapType::Unknown,
            _ => {} // Ignore other events
        }
        buf.clear();
    }

    SitemapType::Unknown
}

/// Extracts all <loc> URLs from a sitemap.xml string
pub fn extract_sitemap_urls(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut urls = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"loc" => {
                // Read the next text event which contains the URL
                if let Ok(Event::Text(e)) = reader.read_event_into(&mut buf) {
                    if let Ok(url) = e.unescape() {
                        urls.push(url.into_owned());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear(); // Clear buffer for the next event
    }

    urls
}
// endregion

/// Fetches URLs concurrently from the sitemap and generates a report.
///
/// # Arguments
///
/// * `urls` - A vector of URL strings fetched from the sitemap.
/// * `client` - A shared, configured HTTP client.
/// * `semaphore` - A semaphore controlling the concurrency level.
/// * `options` - CLI options controlling aspects like output directory and request modifications.
/// * `start_time` - The time when the fetching started, used to calculate elapsed time.
///
/// # Returns
///
/// A `Result` containing a fully populated `Report` if successful, or an error otherwise.
pub async fn fetch_and_generate_report(
    urls: Vec<String>,
    client: Arc<Client>,
    options: &Cli,
    start_time: Instant,
) -> Result<Report, Box<dyn Error>> {
    // Setup progress bars.
    let semaphore = Arc::new(Semaphore::new(options.concurrency_limit as usize));
    let wrapper_pb = indicatif::MultiProgress::new();
    let loading_pb = wrapper_pb.add(indicatif::ProgressBar::new(urls.len() as u64));
    loading_pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(concat!(
                "\x1b[2m[3/3]\x1b[0m",
                " 📥 [{elapsed_precise}] [{bar:40.cyan/blue}] Loading URL {pos}/{len} {msg}"
            ))
            .unwrap()
            .progress_chars("■┄"),
    );

    // Limit to a subset of URLs for demonstration purposes.
    let fetches = urls.iter().map(|u| {
        let semaphore = Arc::clone(&semaphore);
        let client = Arc::clone(&client);
        let output_dir = options.output_dir.clone();
        let mut url = u.clone();

        // Create per-request progress indicators.
        let loading_pb = loading_pb.clone();
        let line_pb = wrapper_pb.add(indicatif::ProgressBar::new_spinner());

        // Append a random timestamp if the option is enabled.
        if options.append_timestamp {
            url = format!("{}?ts={}", url, utils::generate_random_number(10));
        }

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("Semaphore closed");
            line_pb.set_message(format!("Fetching: {}", utils::truncate_message(&url, 80)));
            line_pb.enable_steady_tick(Duration::from_millis(100));
            let result = get_url_response(&url, &client, &output_dir).await;
            line_pb.finish_and_clear();
            loading_pb.inc(1);
            result
        })
    });

    let results: Vec<_> = join_all(fetches).await;
    loading_pb.finish_with_message("- 🏁 Complete!");

    // Process the results and aggregate the responses.
    let mut report = Report {
        sitemap_url: options.sitemap_url.to_string(),
        concurrency_limit: options.concurrency_limit,
        total_time: start_time.elapsed(),
        responses: std::collections::VecDeque::new(),
    };

    report.responses = results
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect();

    Ok(report)
}
