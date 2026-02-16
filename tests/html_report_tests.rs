use std::fs;
use std::process::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("siteprobe_html_test_{}_", prefix))
        .tempdir()
        .expect("Failed to create temp dir")
}

/// Build CLI args with an HTML report path.
fn build_cli_args(sitemap_url: &str, html_report: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--".to_string(),
        sitemap_url.to_string(),
        "--user-agent".to_string(),
        "test-agent".to_string(),
        "--request-timeout".to_string(),
        "10".to_string(),
        "--concurrency-limit".to_string(),
        "5".to_string(),
        "--rate-limit".to_string(),
        "600/1m".to_string(),
        "--report-path-html".to_string(),
        html_report.to_string(),
    ]
}

/// Set up a wiremock server with a valid sitemap and mock pages, returning (server, sitemap_url).
async fn setup_mock_server() -> (MockServer, String) {
    let mock_server = MockServer::start().await;

    let sitemap_xml = include_str!("fixtures/sitemap_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap_xml))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Home page</body></html>"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/catalog"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Catalog page</body></html>"),
        )
        .mount(&mock_server)
        .await;

    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    (mock_server, sitemap_url)
}

#[tokio::test]
async fn test_html_report_is_created() {
    let (_server, sitemap_url) = setup_mock_server().await;
    let tmp = temp_dir("created");
    let html_path = tmp.path().join("report.html");

    let args = build_cli_args(&sitemap_url, html_path.to_str().unwrap());
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe");

    assert!(
        output.status.success(),
        "Command failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(html_path.exists(), "HTML report file should be created");
    let content = fs::read_to_string(&html_path).unwrap();
    assert!(
        content.starts_with("<!DOCTYPE html>"),
        "File should be valid HTML"
    );
}

#[tokio::test]
async fn test_html_report_is_self_contained() {
    let (_server, sitemap_url) = setup_mock_server().await;
    let tmp = temp_dir("self_contained");
    let html_path = tmp.path().join("report.html");

    let args = build_cli_args(&sitemap_url, html_path.to_str().unwrap());
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe");

    assert!(output.status.success(), "Command failed");

    let content = fs::read_to_string(&html_path).unwrap();

    // Self-contained: inline <style> and <script> tags, no external references
    assert!(
        content.contains("<style>"),
        "HTML should contain an inline <style> tag"
    );
    assert!(
        content.contains("<script>"),
        "HTML should contain an inline <script> tag"
    );
    assert!(
        !content.contains("<link rel=\"stylesheet\""),
        "HTML should not reference external stylesheets"
    );
    assert!(
        !content.contains("<script src="),
        "HTML should not reference external scripts"
    );
}

#[tokio::test]
async fn test_html_report_contains_key_sections() {
    let (_server, sitemap_url) = setup_mock_server().await;
    let tmp = temp_dir("sections");
    let html_path = tmp.path().join("report.html");

    let args = build_cli_args(&sitemap_url, html_path.to_str().unwrap());
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe");

    assert!(output.status.success(), "Command failed");

    let content = fs::read_to_string(&html_path).unwrap();

    // Summary statistics cards
    assert!(
        content.contains("Total Requests"),
        "Should contain Total Requests summary card"
    );
    assert!(
        content.contains("Requests/sec"),
        "Should contain Requests/sec summary card"
    );
    assert!(
        content.contains("Success Rate"),
        "Should contain Success Rate summary card"
    );

    // Response Time Statistics section
    assert!(
        content.contains("Response Time Statistics"),
        "Should contain Response Time Statistics section"
    );

    // Performance Statistics section
    assert!(
        content.contains("Performance Statistics"),
        "Should contain Performance Statistics section"
    );

    // Response Time Distribution histogram (SVG)
    assert!(
        content.contains("Response Time Distribution"),
        "Should contain Response Time Distribution chart heading"
    );
    assert!(
        content.contains("<svg") && content.contains("viewBox"),
        "Should contain inline SVG charts"
    );

    // Status Code Breakdown chart (SVG)
    assert!(
        content.contains("Status Code Breakdown"),
        "Should contain Status Code Breakdown chart heading"
    );

    // Response table
    assert!(
        content.contains("<table id=\"responses\">"),
        "Should contain the responses table"
    );
    assert!(
        content.contains("URL") && content.contains("Time (ms)") && content.contains("Status"),
        "Table should have expected column headers"
    );
}

#[tokio::test]
async fn test_html_report_contains_url_data() {
    let (_server, sitemap_url) = setup_mock_server().await;
    let tmp = temp_dir("url_data");
    let html_path = tmp.path().join("report.html");

    let args = build_cli_args(&sitemap_url, html_path.to_str().unwrap());
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe");

    assert!(output.status.success(), "Command failed");

    let content = fs::read_to_string(&html_path).unwrap();

    // The sitemap contains URLs with /catalog and / paths on the mock server.
    // The HTML report should contain the actual URLs from the test.
    assert!(
        content.contains("/catalog"),
        "HTML should contain the /catalog URL path from test data"
    );

    // Verify the sitemap URL appears in the report header/subtitle
    assert!(
        content.contains("/sitemap.xml"),
        "HTML should reference the sitemap URL"
    );

    // Verify status codes appear (all should be 200)
    assert!(
        content.contains(">200<"),
        "HTML should contain 200 status codes in the table"
    );

    // Verify the report has the correct number of table rows (5 URLs in sitemap_valid.xml)
    let row_count = content.matches("<tr><td class=\"url-cell\">").count();
    assert_eq!(
        row_count, 5,
        "HTML table should have 5 data rows matching the sitemap URLs"
    );
}

#[tokio::test]
async fn test_html_report_tilde_expansion() {
    let (_server, sitemap_url) = setup_mock_server().await;

    // Use a tilde path; the CLI should expand ~ to the home directory.
    let home = std::env::var("HOME").expect("HOME not set");
    let unique_name = format!("siteprobe_html_tilde_test_{}.html", std::process::id());
    let tilde_path = format!("~/{}", unique_name);
    let expanded_path = format!("{}/{}", home, unique_name);

    let args = build_cli_args(&sitemap_url, &tilde_path);
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe");

    // Clean up regardless of outcome
    let cleanup = || {
        let _ = fs::remove_file(&expanded_path);
    };

    if !output.status.success() {
        cleanup();
        panic!(
            "Command failed: stdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert!(
        std::path::Path::new(&expanded_path).exists(),
        "Tilde-expanded HTML report should exist at {}",
        expanded_path
    );

    let content = fs::read_to_string(&expanded_path).unwrap();
    assert!(
        content.contains("<!DOCTYPE html>"),
        "Tilde-expanded file should be a valid HTML report"
    );

    cleanup();
}
