use crate::options::Cli;
use crate::report::Response;
use crate::storage::store_response_on_disk;
use base64::Engine;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

/// Builds and configures the HTTP client based on the provided CLI options.
///
/// # Arguments
///
/// * `options` - A reference to the CLI options containing client configuration settings.
///
/// # Returns
///
/// A `Result` containing the built `Client` if successful, or an error otherwise.
pub fn build_client(options: &Cli) -> Result<reqwest::Client, Box<dyn Error>> {
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(options.user_agent.as_str())
        .timeout(Duration::from_secs(options.request_timeout));

    if options.follow_redirects {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::limited(10));
    }

    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(auth) = &options.basic_auth {
        if !auth.is_empty() {
            let encoded_credentials =
                base64::engine::general_purpose::STANDARD.encode(auth.as_bytes());
            let auth_value = format!("Basic {}", encoded_credentials).parse()?;
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
        }
    }

    for header_str in &options.headers {
        if let Some(colon_pos) = header_str.find(':') {
            let name = header_str[..colon_pos].trim();
            let value = header_str[colon_pos + 1..].trim();
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(header_name, header_value);
            }
        }
    }

    if !headers.is_empty() {
        client_builder = client_builder.default_headers(headers);
    }

    Ok(client_builder.build()?)
}

/// Fetches the given URL and captures the response metadata.
///
/// This asynchronous function makes a GET request to the specified URL and captures:
/// - The HTTP status code of the response.
/// - The size of the response body (in bytes).
/// - The total duration it took to complete the request.
///
/// If `output_dir` is set, the response body is also stored on disk.
///
/// # Arguments
///
/// * `url` - A string slice representing the URL to fetch.
/// * `client` - A reference to a `reqwest::Client` instance used to perform the request.
/// * `output_dir` - An optional directory where the response body is stored on disk.
///
/// # Errors
///
/// Common request failures are mapped to standardized HTTP status codes instead of
/// errors (408 for timeouts, 502 for connection errors, 400 for malformed requests),
/// so they show up as regular entries in the report. Any other error (e.g. exceeding
/// the redirect limit) is propagated as `Err(reqwest::Error)`.
pub async fn get_url_response(
    url: &str,
    client: &reqwest::Client,
    output_dir: Option<&Path>,
) -> Result<Response, reqwest::Error> {
    let start_time = tokio::time::Instant::now();
    let response = client.get(url).send().await;

    // `final_url` is the URL the response actually came from (it may differ from
    // the requested URL when redirects are followed). It is `None` when the
    // request failed before receiving a response.
    let (status, final_url, body) = match response {
        Ok(resp) => {
            let final_url = Some(resp.url().clone());
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            (status, final_url, Some(body))
        }
        Err(e) if e.is_timeout() => (reqwest::StatusCode::REQUEST_TIMEOUT, None, None),
        Err(e) if e.is_connect() => (reqwest::StatusCode::BAD_GATEWAY, None, None),
        Err(e) if e.is_request() => (reqwest::StatusCode::BAD_REQUEST, None, None),
        Err(e) => return Err(e),
    };

    let response_size = body.as_ref().map_or(0, String::len);

    if let (Some(output_dir), Some(final_url), Some(body)) =
        (output_dir, final_url.as_ref(), body.as_deref())
    {
        store_response_on_disk(output_dir, final_url, body).await;
    }

    Ok(Response {
        response_time: start_time.elapsed(),
        response_size,
        url: final_url.map_or_else(|| url.to_string(), |u| u.to_string()),
        status_code: status,
    })
}
