use std::path::Path;
use tokio::fs;

/// Stores the HTTP response body on disk as an HTML file.
///
/// The file name is derived from the URL's path with `.html` appended; if the
/// URL path is empty, the file is named `index.html`. Any necessary parent
/// directories are created if they don't already exist.
///
/// A write failure is reported on stderr but does not abort the probe run.
///
/// # Arguments
///
/// * `storage_path` - The target directory where the response will be stored.
/// * `url` - The URL the response was fetched from.
/// * `body` - The response body content that will be written to the file.
pub async fn store_response_on_disk(storage_path: &Path, url: &url::Url, body: &str) {
    let document_path = format!(
        "{}.html",
        if url.path().trim_matches('/').is_empty() {
            "index"
        } else {
            url.path().trim_matches('/')
        }
    );
    let target_path = storage_path.join(document_path);

    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }

    if let Err(e) = fs::write(&target_path, body).await {
        eprintln!("❌ Failed to write document to disk: {}", e);
    }
}
