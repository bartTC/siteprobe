use crate::network::get_url_response;
use crate::options::Cli;
use crate::report::Report;
use crate::utils;
use console::style;
use flate2::read::GzDecoder;
use futures::future::join_all;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Client;
use std::error::Error;
use std::fmt;
use std::io::Read;
use std::num::NonZeroU32;
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

/// A non-keyed, in-memory rate limiter shared by all request tasks.
type DirectRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

// Implement Display for SitemapType
impl fmt::Display for SitemapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
// endregion

// region: Functions

/// Decompresses gzip-compressed bytes into a UTF-8 string.
pub fn decompress_gzip(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    Ok(decompressed)
}

/// Checks if the content is gzip-compressed, either by URL suffix
/// or by inspecting the gzip magic bytes (0x1f, 0x8b).
pub fn is_gzip_content(url: &str, bytes: &[u8]) -> bool {
    if url.ends_with(".gz") {
        return true;
    }
    // Check for gzip magic number
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

/// Fetches a sitemap URL, automatically decompressing gzip content if detected.
async fn get_sitemap_content(url: &str, client: &Client) -> Result<String, Box<dyn Error>> {
    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;

    if is_gzip_content(url, &bytes) {
        decompress_gzip(&bytes)
    } else {
        Ok(String::from_utf8(bytes.to_vec())?)
    }
}

pub async fn get_sitemap_urls(
    sitemap_url: &str,
    client: &Client,
    quiet: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let content = match get_sitemap_content(sitemap_url, client).await {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Unable to fetch sitemap: {}", e).into());
        }
    };

    let sitemap_type = identify_sitemap_type(&content);
    if !quiet {
        println!("{} 🔎 Fetch {}...", style("[1/3]").dim(), sitemap_type);
    }

    if sitemap_type == SitemapType::Unknown {
        return Err(format!("The sitemap does not contain any URLs: {}", sitemap_url).into());
    }

    // A sitemap.xml file might be an index file, linking to other sitemaps.
    // In that case, retrieve the urls from all those sitemaps.
    let mut urls = Vec::new();

    if !quiet {
        println!(
            "{} 🚚 Collect all URLs from sitemap...",
            style("[2/3]").dim()
        );
    }
    if sitemap_type == SitemapType::SitemapIndex {
        let sitemap_urls = extract_sitemap_urls(&content);
        for sitemap_url in sitemap_urls {
            match get_sitemap_content(&sitemap_url, client).await {
                Ok(content) => {
                    urls.extend(extract_sitemap_urls(&content));
                }
                Err(_) => {
                    eprintln!(
                        "{} The referenced sitemap is missing: {}",
                        style("[ERROR]").red(),
                        sitemap_url
                    );
                }
            };
        }
    } else if sitemap_type == SitemapType::UrlSet {
        urls.extend(extract_sitemap_urls(&content));
    }

    // Deduplicate URLs - a URL might appear in multiple sitemap files
    urls.sort();
    urls.dedup();

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
    client: &Arc<Client>,
    options: &Cli,
    start_time: &Instant,
) -> Result<Report, Box<dyn Error>> {
    // Setup concurrency
    let semaphore = Arc::new(Semaphore::new(options.concurrency_limit as usize));

    // Setup the rate limiter, paired with its requests-per-minute limit for display.
    let rate_limiter: Option<Arc<(u32, DirectRateLimiter)>> =
        options.rate_limit.map(|requests_per_minute| {
            let quota = NonZeroU32::new(requests_per_minute)
                .expect("rate limit is validated to be at least 1");
            Arc::new((
                requests_per_minute,
                RateLimiter::direct(Quota::per_minute(quota).allow_burst(NonZeroU32::MIN)),
            ))
        });

    // Setup progress bars.
    let wrapper_pb = indicatif::MultiProgress::new();
    if options.json {
        wrapper_pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }
    let loading_pb = wrapper_pb.add(indicatif::ProgressBar::new(urls.len() as u64));
    loading_pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(concat!(
                "\x1b[2m[3/3]\x1b[0m",
                " 📥 [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} (ETA: {eta_precise}) {msg}"
            ))
            .unwrap()
            .progress_chars("■┄"),
    );

    let retries = options.retries;

    let fetches = urls.iter().map(|u| {
        let semaphore = Arc::clone(&semaphore);
        let rate_limiter = rate_limiter.clone();
        let client = Arc::clone(client);
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

            if let Some(rate) = rate_limiter.as_deref() {
                let (limit, limiter) = rate;

                // Set the progress bar message to indicate rate limiting
                line_pb.set_message(format!(
                    "Waiting for rate limit ({}/min): {}",
                    limit,
                    utils::truncate_message(&url, 80)
                ));

                // Wait until the rate limit is satisfied
                limiter.until_ready().await;
            }

            line_pb.set_message(format!("Fetching: {}", utils::truncate_message(&url, 80)));
            line_pb.enable_steady_tick(Duration::from_millis(100));

            let mut result = get_url_response(&url, &client, output_dir.as_deref()).await;

            // Retry logic: retry on network errors or 5xx status codes
            for attempt in 1..=retries {
                let should_retry = match &result {
                    Ok(resp) => resp.status_code.is_server_error(),
                    Err(_) => true,
                };

                if !should_retry {
                    break;
                }

                line_pb.set_message(format!(
                    "Retrying ({}/{}): {}",
                    attempt,
                    retries,
                    utils::truncate_message(&url, 70)
                ));
                tokio::time::sleep(Duration::from_secs(1)).await;
                result = get_url_response(&url, &client, output_dir.as_deref()).await;
            }

            line_pb.finish_and_clear();
            loading_pb.inc(1);
            result
        })
    });

    let results: Vec<_> = join_all(fetches).await;
    loading_pb.finish_with_message("- 🏁 Complete!");

    // Aggregate the responses. Failures that could not be mapped to a status
    // code are reported on stderr instead of being silently dropped.
    let mut responses = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(Ok(response)) => responses.push(response),
            Ok(Err(e)) => eprintln!("{} Request failed: {}", style("[ERROR]").red(), e),
            Err(e) => eprintln!("{} Request task failed: {}", style("[ERROR]").red(), e),
        }
    }

    Ok(Report {
        sitemap_url: options.sitemap_url.to_string(),
        concurrency_limit: options.concurrency_limit,
        rate_limit: options.rate_limit,
        total_time: start_time.elapsed(),
        responses,
    })
}
