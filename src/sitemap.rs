use crate::network::get_url_response;
use crate::options::{Cli, UrlSource};
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
use quick_xml::name::QName;
use reqwest::Client;
use std::error::Error;
use std::fmt;
use std::io::Read;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::Instant;
use url::Url;

// region: Structs & Enums

/// The kind of document a URL source turned out to contain.
#[derive(Debug, PartialEq)]
pub enum SitemapType {
    /// A `<sitemapindex>` referencing other sitemaps.
    SitemapIndex,
    /// A `<urlset>` listing page URLs.
    UrlSet,
    /// A plain-text list with one URL per line.
    PlainText,
    /// Neither a sitemap nor a URL list.
    Unknown,
}

impl fmt::Display for SitemapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            SitemapType::SitemapIndex => "sitemap index",
            SitemapType::UrlSet => "sitemap",
            SitemapType::PlainText => "URL list",
            SitemapType::Unknown => "unknown document",
        };
        write!(f, "{label}")
    }
}

/// Result of parsing a plain-text URL list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UrlList {
    /// Lines that parsed as absolute http(s) URLs, in input order.
    pub urls: Vec<String>,
    /// Non-blank, non-comment lines that are not valid http(s) URLs.
    pub invalid_lines: Vec<String>,
}

/// A non-keyed, in-memory rate limiter shared by all request tasks.
type DirectRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;
// endregion

// region: Functions

/// Decompresses gzip-compressed bytes into a UTF-8 string.
pub fn decompress_gzip(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    Ok(decompressed)
}

/// Checks if the content is gzip-compressed, either by the `.gz` suffix of
/// its name (URL or file path) or by inspecting the gzip magic bytes (0x1f, 0x8b).
pub fn is_gzip_content(name: &str, bytes: &[u8]) -> bool {
    if name.ends_with(".gz") {
        return true;
    }
    // Check for gzip magic number
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

/// Reads the raw bytes of a URL source.
async fn read_source(source: &UrlSource, client: &Client) -> Result<Vec<u8>, Box<dyn Error>> {
    match source {
        UrlSource::Remote(url) => {
            let response = client.get(url.as_str()).send().await?.error_for_status()?;
            Ok(response.bytes().await?.to_vec())
        }
        UrlSource::File(path) => Ok(tokio::fs::read(path).await?),
        UrlSource::Stdin => {
            // Stdin is read once, up front, before any concurrent work starts,
            // so a blocking read is fine here.
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            Ok(bytes)
        }
    }
}

/// Loads a URL source as text, automatically decompressing gzip content if detected.
async fn get_source_content(source: &UrlSource, client: &Client) -> Result<String, Box<dyn Error>> {
    let bytes = read_source(source, client).await?;

    if is_gzip_content(&source.to_string(), &bytes) {
        decompress_gzip(&bytes)
    } else {
        Ok(String::from_utf8(bytes)?)
    }
}

/// Fetches a sitemap referenced by URL from a sitemap index.
async fn get_remote_content(url: &str, client: &Client) -> Result<String, Box<dyn Error>> {
    let source = UrlSource::Remote(Url::parse(url)?);
    get_source_content(&source, client).await
}

/// Assembles the full list of URLs to probe: the `--url` values plus, if a
/// source was given, every URL extracted from it. The result is sorted and
/// deduplicated.
pub async fn collect_urls(options: &Cli, client: &Client) -> Result<Vec<String>, Box<dyn Error>> {
    let quiet = options.json;
    let mut urls: Vec<String> = options.urls.iter().map(Url::to_string).collect();

    match &options.source {
        Some(source) => urls.extend(get_source_urls(source, client, quiet).await?),
        None => {
            if !quiet {
                println!(
                    "{} 🔎 Using URLs from the command line...",
                    style("[1/3]").dim()
                );
                println!("{} 🚚 Collect all URLs...", style("[2/3]").dim());
            }
        }
    }

    // Deduplicate URLs - a URL might appear in multiple sitemap files, or
    // both in the source and on the command line.
    urls.sort();
    urls.dedup();

    Ok(urls)
}

/// Loads a source and extracts all URLs from it, following sitemap index
/// references to other sitemaps.
async fn get_source_urls(
    source: &UrlSource,
    client: &Client,
    quiet: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let content = match get_source_content(source, client).await {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Unable to load {}: {}", source, e).into());
        }
    };

    let sitemap_type = identify_sitemap_type(&content);
    if !quiet {
        println!("{} 🔎 Fetch {}...", style("[1/3]").dim(), sitemap_type);
    }

    if sitemap_type == SitemapType::Unknown {
        return Err(format!(
            "The source does not contain any URLs (expected a sitemap.xml or a plain-text list of URLs): {}",
            source
        )
        .into());
    }

    let mut urls = Vec::new();

    if !quiet {
        println!("{} 🚚 Collect all URLs...", style("[2/3]").dim());
    }
    match sitemap_type {
        // A sitemap.xml file might be an index file, linking to other sitemaps.
        // In that case, retrieve the urls from all those sitemaps.
        SitemapType::SitemapIndex => {
            for sitemap_url in extract_sitemap_urls(&content) {
                match get_remote_content(&sitemap_url, client).await {
                    Ok(content) => urls.extend(extract_sitemap_urls(&content)),
                    Err(_) => {
                        eprintln!(
                            "{} The referenced sitemap is missing: {}",
                            style("[ERROR]").red(),
                            sitemap_url
                        );
                    }
                }
            }
        }
        SitemapType::UrlSet => urls.extend(extract_sitemap_urls(&content)),
        SitemapType::PlainText => {
            let list = parse_url_list(&content);
            for line in &list.invalid_lines {
                eprintln!(
                    "{} Skipping line that is not an http(s) URL: {}",
                    style("[WARN]").yellow(),
                    line
                );
            }
            urls.extend(list.urls);
        }
        SitemapType::Unknown => unreachable!("handled above"),
    }

    Ok(urls)
}

/// Classifies content as a sitemap index, a sitemap, or a plain-text URL list.
pub fn identify_sitemap_type(content: &str) -> SitemapType {
    if let Some(sitemap_type) = identify_xml_root(content) {
        return sitemap_type;
    }

    if parse_url_list(content).urls.is_empty() {
        SitemapType::Unknown
    } else {
        SitemapType::PlainText
    }
}

/// Classifies XML content by its root element. Returns `None` if there is no
/// root element at all, i.e. the content is not XML.
fn identify_xml_root(xml: &str) -> Option<SitemapType> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                return Some(match e.name().as_ref() {
                    "sitemapindex" => SitemapType::SitemapIndex,
                    "urlset" => SitemapType::UrlSet,
                    _ => SitemapType::Unknown,
                });
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {} // Ignore other events
        }
        buf.clear();
    }
}

/// Parses a plain-text URL list: one URL per line. Blank lines and lines
/// starting with `#` are ignored. Valid URLs are normalized through `Url`,
/// so they compare equal to the `--url` values.
pub fn parse_url_list(text: &str) -> UrlList {
    let mut list = UrlList::default();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match Url::parse(line) {
            Ok(url) if utils::is_http_url(&url) => list.urls.push(url.to_string()),
            _ => list.invalid_lines.push(line.to_string()),
        }
    }

    list
}

/// Extracts all <loc> URLs from a sitemap.xml string
pub fn extract_sitemap_urls(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut urls = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "loc" => {
                // Read everything up to the closing </loc> tag. Text events are
                // split at entity references, so a single Text event would lose
                // everything past the first `&amp;`.
                if let Ok(text) = reader.read_text(QName("loc")) {
                    let decoded = text.xml_content(quick_xml::XmlVersion::Implicit1_0);
                    if let Ok(url) = quick_xml::escape::unescape(&decoded) {
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

/// Fetches all collected URLs concurrently and generates a report.
///
/// # Arguments
///
/// * `urls` - The URLs to probe, as assembled by [`collect_urls`].
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
        source: options.source_label(),
        concurrency_limit: options.concurrency_limit,
        rate_limit: options.rate_limit,
        total_time: start_time.elapsed(),
        responses,
    })
}
