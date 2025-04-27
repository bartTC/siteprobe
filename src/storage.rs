use std::fs;
use std::path;

/// Stores the HTTP response body on disk as an HTML file.
///
/// This function takes the storage path, the URL from which the response was fetched,
/// and the response body, and writes the body to a file located in the specified
/// storage path. The file name is generated based on the URL's path. If the URL path
/// is empty, the file is named `index.html`, otherwise, the file name is derived
/// from the URL path with `.html` as its extension. Any necessary directories in the
/// path are created if they don't already exist.
///
/// # Arguments
///
/// * `storage_path` - A reference to the target directory where the response will be stored.
///   This should be passed as an `&Path` (not `&PathBuf` for efficiency).
/// * `url` - A reference to the URL object representing the source of the response.
/// * `body` - The response body content that will be written to the file.
///
/// # Panics
///
/// This function will panic if it fails to write the file to the specified path.
pub async fn store_response_on_disk(storage_path: &path::Path, url: &url::Url, body: &str) {
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
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(target_path, body) {
        Ok(_) => (),
        Err(e) => eprintln!("❌ Failed to write document to disk: {}", e),
    }
}
