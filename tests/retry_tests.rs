use std::process::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a minimal sitemap XML with a single URL.
fn single_url_sitemap(url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{}</loc></url>
</urlset>"#,
        url
    )
}

/// Run siteprobe via `cargo run` with --json output and return the process Output.
fn run_siteprobe(sitemap_url: &str, retries: u8) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            sitemap_url,
            "--json",
            "--retries",
            &retries.to_string(),
            "--request-timeout",
            "5",
            "--concurrency-limit",
            "1",
        ])
        .output()
        .expect("Failed to execute siteprobe binary")
}

// ---------------------------------------------------------------------------
// Test 1: --retries 0 (default) does NOT retry failed requests
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_retries_zero_no_retry_on_500() {
    let mock_server = MockServer::start().await;

    let page_url = format!("{}/page", mock_server.uri());
    let sitemap_xml = single_url_sitemap(&page_url);

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(&sitemap_xml))
        .mount(&mock_server)
        .await;

    // The page always returns 500
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1) // Should be called exactly once (no retries)
        .mount(&mock_server)
        .await;

    let output = run_siteprobe(&format!("{}/sitemap.xml", mock_server.uri()), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let responses = json["responses"]
        .as_array()
        .expect("responses should be an array");
    assert_eq!(responses.len(), 1, "Should have exactly 1 response");
    assert_eq!(
        responses[0]["statusCode"].as_u64().unwrap(),
        500,
        "Status should be 500"
    );
}

// ---------------------------------------------------------------------------
// Test 2: --retries 2 retries on 5xx, eventually succeeds
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_retries_two_succeeds_after_failures() {
    let mock_server = MockServer::start().await;

    let page_url = format!("{}/page", mock_server.uri());
    let sitemap_xml = single_url_sitemap(&page_url);

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(&sitemap_xml))
        .mount(&mock_server)
        .await;

    // First two requests return 500, then 200.
    // wiremock matches mocks in reverse mount order, so mount the success fallback first,
    // then mount the 500 response limited to 2 hits (it will take priority while active).
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    let output = run_siteprobe(&format!("{}/sitemap.xml", mock_server.uri()), 2);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let responses = json["responses"]
        .as_array()
        .expect("responses should be an array");
    assert_eq!(responses.len(), 1, "Should have exactly 1 response");
    assert_eq!(
        responses[0]["statusCode"].as_u64().unwrap(),
        200,
        "Final status should be 200 after retries"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Retries do NOT happen on 4xx (client errors)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_no_retry_on_4xx() {
    let mock_server = MockServer::start().await;

    let page_url = format!("{}/page", mock_server.uri());
    let sitemap_xml = single_url_sitemap(&page_url);

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(&sitemap_xml))
        .mount(&mock_server)
        .await;

    // The page returns 404. With retries=2, it should still only be called once
    // because 4xx responses are not retried.
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .expect(1) // Exactly 1 call — no retries for client errors
        .mount(&mock_server)
        .await;

    let output = run_siteprobe(&format!("{}/sitemap.xml", mock_server.uri()), 2);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let responses = json["responses"]
        .as_array()
        .expect("responses should be an array");
    assert_eq!(responses.len(), 1);
    assert_eq!(
        responses[0]["statusCode"].as_u64().unwrap(),
        404,
        "Status should be 404"
    );
}

// ---------------------------------------------------------------------------
// Test 4: CLI validation rejects --retries 11 (max is 10)
// ---------------------------------------------------------------------------
#[test]
fn test_retries_max_validation() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--retries",
            "11",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        !output.status.success(),
        "Should fail when --retries exceeds maximum of 10"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("11") || stderr.contains("invalid") || stderr.contains("not in"),
        "Error should mention the invalid value: {}",
        stderr
    );
}
