use crate::options::Cli;
use crate::report::Response;
use crate::storage::store_response_on_disk;
use base64::Engine;
use std::error::Error;
use std::path::PathBuf;
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
        .user_agent(options.user_agent.as_str())
        .timeout(Duration::from_secs(options.request_timeout as u64));

    if options.follow_redirects {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::limited(10));
    }

    if let Some(auth) = &options.basic_auth {
        if !auth.is_empty() {
            let mut headers = reqwest::header::HeaderMap::new();
            let encoded_credentials =
                base64::engine::general_purpose::STANDARD.encode(auth.as_bytes());
            let auth_value = format!("Basic {}", encoded_credentials).parse()?;
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            client_builder = client_builder.default_headers(headers);
        }
    }
    Ok(client_builder.build()?)
}

/// Fetches the content of a given URL as a `String`.
///
/// This function sends a GET request to the specified URL using the provided
/// asynchronous HTTP client (`reqwest::Client`). It ensures the response has
/// a successful HTTP status code, then retrieves the response body as text
/// and returns it.
///
/// # Arguments
///
/// * `url` - A string slice that holds the URL to be fetched.
/// * `client` - A reference to a `reqwest::Client` used to perform the HTTP request.
///
/// # Returns
///
/// On success, returns a `Result` containing the content of the URL as a `String`.
/// On failure, returns a `reqwest::Error` wrapped in a `Result::Err`.
///
/// # Errors
///
/// This function will return an error if:
/// - The GET request fails (e.g., network issues).
/// - The HTTP response status is not successful (e.g., 4xx or 5xx error).
/// - The response body cannot be converted to text.
pub async fn get_url_content(
    url: &str,
    client: &reqwest::Client,
) -> Result<String, reqwest::Error> {
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
}

/// Fetches the content at the specified URL using the given HTTP client.
///
/// This asynchronous function makes a GET request to the specified URL and captures:
/// - The HTTP status code of the response.
/// - The size of the response content (in bytes).
/// - The total duration it took to complete the request.
///
/// # Parameters
/// - `url`: A string slice representing the URL to fetch.
/// - `client`: A reference to a `reqwest::Client` instance used to perform the request.
///
/// # Returns
/// Returns a `Result` containing a [`Response`](crate::report::Response) struct with the
/// request metadata on success, or a boxed error (`Box<dyn Error + Send + Sync>`) on failure.
///
/// # Error Handling
/// In case of an HTTP error, such as connection issues, request timeouts, or client-related
/// errors (e.g., malformed request), this function returns standardized HTTP status codes
/// (e.g., 408 for timeout, 502 for connection errors, etc.).
/// Any unexpected errors are propagated as `Err(Box<dyn Error + Send + Sync>)`.
pub async fn get_url_response(
    url: &str,
    client: &reqwest::Client,
    output_dir: &Option<PathBuf>,
) -> Result<Response, reqwest::Error> {
    let start_time = tokio::time::Instant::now();
    let response = client.get(url).send().await;

    let (status, url, content_length, body) = match response {
        Ok(resp) => {
            let url = Some(resp.url().clone());
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            (status, url, body.len(), Some(body))
        }
        Err(e) if e.is_timeout() => (reqwest::StatusCode::REQUEST_TIMEOUT, None, 0, None),
        Err(e) if e.is_connect() => (reqwest::StatusCode::BAD_GATEWAY, None, 0, None),
        Err(e) if e.is_request() => (reqwest::StatusCode::BAD_REQUEST, None, 0, None),
        Err(e) => return Err(e),
    };

    if let (Some(output_dir), Some(url_ref)) = (output_dir, url.as_ref()) {
        store_response_on_disk(output_dir, url_ref, body.unwrap_or_default().as_str()).await;
    }

    Ok(Response {
        response_time: start_time.elapsed(),
        response_size: content_length,
        url: url.unwrap().to_string(),
        status_code: status,
    })
}
