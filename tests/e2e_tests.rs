use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("siteprobe_test_{}_", prefix))
        .tempdir()
        .expect("Failed to create temp dir")
}

/// Helper to build common CLI arguments for E2E tests
fn build_cli_args(
    sitemap_url: &str,
    csv_report: Option<&str>,
    json_report: Option<&str>,
    output_dir: Option<&str>,
    basic_auth: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
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
        "--slow-num".to_string(),
        "5".to_string(),
        "--rate-limit".to_string(),
        "600/1m".to_string(),
        "--append-timestamp".to_string(),
    ];

    if let Some(auth) = basic_auth {
        args.push("--basic-auth".to_string());
        args.push(auth.to_string());
    }

    if let Some(csv) = csv_report {
        args.push("--report-path".to_string());
        args.push(csv.to_string());
    }

    if let Some(json) = json_report {
        args.push("--report-path-json".to_string());
        args.push(json.to_string());
    }

    if let Some(dir) = output_dir {
        args.push("--output-dir".to_string());
        args.push(dir.to_string());
    }

    args
}

#[tokio::test]
async fn test_e2e_valid_sitemap() {
    // True E2E test: single valid sitemap file (not an index)

    let mock_server = MockServer::start().await;

    // Use valid single sitemap
    let sitemap_xml = include_str!("fixtures/sitemap_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap_xml))
        .mount(&mock_server)
        .await;

    // Mock the page URLs from the sitemap
    // Note: We need to mock the root path separately
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Home page</body></html>"),
        )
        .mount(&mock_server)
        .await;

    // Mock /catalog path (will match all query string variations)
    Mock::given(method("GET"))
        .and(path("/catalog"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Catalog page</body></html>"),
        )
        .mount(&mock_server)
        .await;

    // Create temporary directory for test outputs
    let temp_dir = temp_dir("single_sitemap");
    let csv_report = temp_dir.path().join("report.csv");
    let json_report = temp_dir.path().join("report.json");
    let output_dir = temp_dir.path().join("pages");

    // Run the actual CLI binary
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_cli_args(
        &sitemap_url,
        Some(csv_report.to_str().unwrap()),
        Some(json_report.to_str().unwrap()),
        Some(output_dir.to_str().unwrap()),
        None,
    );
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "Command failed with status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output contains expected information
    assert!(
        stdout.contains("Total Requests Processed"),
        "Output should contain 'Total Requests Processed'"
    );
    assert!(
        stdout.contains("Success Rate"),
        "Output should contain 'Success Rate'"
    );

    // Should have processed 5 URLs successfully
    assert!(stdout.contains("100"), "Should have 100% success rate");

    // Verify CSV report was created and contains data
    assert!(csv_report.exists(), "CSV report should be created");
    let csv_content = fs::read_to_string(&csv_report).expect("Failed to read CSV report");
    assert!(
        csv_content.contains("URL") && csv_content.contains("Response Time"),
        "CSV should have header"
    );
    // CSV should have 6 lines (1 header + 5 data rows)
    assert_eq!(csv_content.lines().count(), 6, "CSV should have 6 lines");

    // Verify JSON report was created and is valid JSON
    assert!(json_report.exists(), "JSON report should be created");
    let json_content = fs::read_to_string(&json_report).expect("Failed to read JSON report");
    let json: serde_json::Value =
        serde_json::from_str(&json_content).expect("JSON should be valid");
    assert_eq!(
        json["responses"].as_array().unwrap().len(),
        5,
        "JSON should have 5 responses"
    );

    // Verify output directory was created with downloaded pages
    // Note: Only 2 files because URLs with same path but different query strings
    // overwrite each other when saved to disk
    assert!(output_dir.exists(), "Output directory should be created");
    let downloaded_files: Vec<_> = fs::read_dir(&output_dir)
        .expect("Failed to read output dir")
        .collect();
    assert!(
        downloaded_files.len() >= 1,
        "Should have at least 1 downloaded page"
    );
}

#[tokio::test]
async fn test_e2e_valid_sitemap_with_basic_auth() {
    // True E2E test: sitemap requiring basic authentication
    use wiremock::matchers::header;

    let mock_server = MockServer::start().await;

    // Use valid single sitemap
    let sitemap_xml = include_str!("fixtures/sitemap_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    // Mock sitemap.xml with basic auth requirement
    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .and(header("authorization", "Basic dGVzdHVzZXI6dGVzdHBhc3M=")) // testuser:testpass
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap_xml))
        .mount(&mock_server)
        .await;

    // Mock sitemap.xml without auth - should return 401
    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    // Mock the page URLs with basic auth
    Mock::given(method("GET"))
        .and(path("/"))
        .and(header("authorization", "Basic dGVzdHVzZXI6dGVzdHBhc3M="))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><body>Protected home page</body></html>"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/catalog"))
        .and(header("authorization", "Basic dGVzdHVzZXI6dGVzdHBhc3M="))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><body>Protected catalog</body></html>"),
        )
        .mount(&mock_server)
        .await;

    // Mock pages without auth - should return 401
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/catalog"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    // Create temporary directory for test outputs
    let temp_dir = temp_dir("basic_auth");
    let csv_report = temp_dir.path().join("report.csv");
    let json_report = temp_dir.path().join("report.json");

    // Run the actual CLI binary with basic auth
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_cli_args(
        &sitemap_url,
        Some(csv_report.to_str().unwrap()),
        Some(json_report.to_str().unwrap()),
        None,
        Some("testuser:testpass"),
    );
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "Command failed with status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output contains expected information
    assert!(
        stdout.contains("Total Requests Processed"),
        "Output should contain 'Total Requests Processed'"
    );
    assert!(
        stdout.contains("Success Rate"),
        "Output should contain 'Success Rate'"
    );

    // Should have processed 5 URLs successfully with auth
    assert!(stdout.contains("100"), "Should have 100% success rate");

    // Verify CSV report was created and contains data
    assert!(csv_report.exists(), "CSV report should be created");
    let csv_content = fs::read_to_string(&csv_report).expect("Failed to read CSV report");
    assert!(
        csv_content.contains("URL") && csv_content.contains("Response Time"),
        "CSV should have header"
    );
    // CSV should have 6 lines (1 header + 5 data rows)
    assert_eq!(csv_content.lines().count(), 6, "CSV should have 6 lines");

    // Verify all responses have 200 status code (authenticated successfully)
    for line in csv_content.lines().skip(1) {
        // Skip header
        assert!(
            line.contains("200"),
            "All requests should return 200 with valid auth"
        );
    }

    // Verify JSON report was created and is valid JSON
    assert!(json_report.exists(), "JSON report should be created");
    let json_content = fs::read_to_string(&json_report).expect("Failed to read JSON report");
    let json: serde_json::Value =
        serde_json::from_str(&json_content).expect("JSON should be valid");
    assert_eq!(
        json["responses"].as_array().unwrap().len(),
        5,
        "JSON should have 5 responses"
    );

    // Verify all responses in JSON are successful
    for response in json["responses"].as_array().unwrap() {
        let status = response["statusCode"].as_u64().unwrap();
        assert_eq!(
            status, 200,
            "All responses should have 200 status with auth"
        );
    }
}

#[tokio::test]
async fn test_e2e_valid_sitemap_index() {
    // True E2E test: invoke CLI binary with mocked network for sitemap index

    let mock_server = MockServer::start().await;

    // Setup sitemap index - replace example.com with mock server URL
    let index_xml = include_str!("fixtures/sitemap_index_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    // Setup individual sitemaps - also replace URLs
    let sitemap1_xml =
        include_str!("fixtures/sitemap1.xml").replace("http://www.example.com", &mock_server.uri());
    let sitemap2_xml =
        include_str!("fixtures/sitemap2.xml").replace("http://www.example.com", &mock_server.uri());
    let sitemap3_xml =
        include_str!("fixtures/sitemap3.xml").replace("http://www.example.com", &mock_server.uri());

    // Mock sitemap index
    Mock::given(method("GET"))
        .and(path("/sitemap_index.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(index_xml))
        .mount(&mock_server)
        .await;

    // Mock individual sitemaps
    Mock::given(method("GET"))
        .and(path("/sitemap1.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap1_xml))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sitemap2.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap2_xml))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sitemap3.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap3_xml))
        .mount(&mock_server)
        .await;

    // Mock the actual page URLs (7 unique pages after deduplication)
    let pages = vec![
        "/page1",
        "/page2",
        "/page3",
        "/page4",
        "/page5",
        "/page6",
        "/shared-page",
    ];

    for page in pages {
        Mock::given(method("GET"))
            .and(path(page))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("<html><body>Content for {}</body></html>", page)),
            )
            .mount(&mock_server)
            .await;
    }

    // Create temporary directory for test outputs
    let temp_dir = temp_dir("sitemap_index");
    let csv_report = temp_dir.path().join("report.csv");
    let json_report = temp_dir.path().join("report.json");
    let output_dir = temp_dir.path().join("pages");

    // Run the actual CLI binary
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    let args = build_cli_args(
        &sitemap_url,
        Some(csv_report.to_str().unwrap()),
        Some(json_report.to_str().unwrap()),
        Some(output_dir.to_str().unwrap()),
        None,
    );
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    // Verify the command succeeded
    assert!(
        output.status.success(),
        "Command failed with status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output contains expected information
    assert!(
        stdout.contains("Total Requests Processed"),
        "Output should contain 'Total Requests Processed'"
    );
    assert!(
        stdout.contains("Success Rate"),
        "Output should contain 'Success Rate'"
    );

    // The report should show 7 URLs processed successfully
    // Verify we got a 100% success rate
    assert!(stdout.contains("100"), "Should have 100% success rate");

    // Verify CSV report was created and contains data
    assert!(csv_report.exists(), "CSV report should be created");
    let csv_content = fs::read_to_string(&csv_report).expect("Failed to read CSV report");
    assert!(
        csv_content.contains("URL") && csv_content.contains("Response Time"),
        "CSV should have header"
    );
    // CSV should have 8 lines (1 header + 7 data rows)
    assert_eq!(csv_content.lines().count(), 8, "CSV should have 8 lines");

    // Verify JSON report was created and is valid JSON
    assert!(json_report.exists(), "JSON report should be created");
    let json_content = fs::read_to_string(&json_report).expect("Failed to read JSON report");
    let json: serde_json::Value =
        serde_json::from_str(&json_content).expect("JSON should be valid");
    assert_eq!(
        json["responses"].as_array().unwrap().len(),
        7,
        "JSON should have 7 responses"
    );

    // Verify output directory was created with downloaded pages
    assert!(output_dir.exists(), "Output directory should be created");
    let downloaded_files: Vec<_> = fs::read_dir(&output_dir)
        .expect("Failed to read output dir")
        .collect();
    assert_eq!(downloaded_files.len(), 7, "Should have 7 downloaded pages");
}

#[tokio::test]
async fn test_e2e_invalid_sitemap_file() {
    // True E2E test: invalid sitemap (RSS feed) should return error

    let mock_server = MockServer::start().await;

    // Use invalid sitemap (RSS feed)
    let invalid_xml = include_str!("fixtures/sitemap_invalid.xml");

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(invalid_xml))
        .mount(&mock_server)
        .await;

    // Run the actual CLI binary
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_cli_args(&sitemap_url, None, None, None, None);
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    // Should fail because sitemap type is Unknown
    assert!(!output.status.success(), "Should fail with invalid sitemap");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not contain any URLs") || stderr.contains("ERROR"),
        "Error should mention no URLs found"
    );
}

#[tokio::test]
async fn test_e2e_invalid_sitemap_index() {
    // True E2E test: sitemap index with all missing sitemaps

    let mock_server = MockServer::start().await;

    // Use invalid index that references non-existent sitemaps
    let index_xml = include_str!("fixtures/sitemap_index_invalid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    // Mock sitemap index
    Mock::given(method("GET"))
        .and(path("/sitemap_index.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(index_xml))
        .mount(&mock_server)
        .await;

    // Don't mock the referenced sitemaps - they'll 404

    // Create temporary directory for test outputs
    let temp_dir = temp_dir("invalid_index");
    let csv_report = temp_dir.path().join("report.csv");
    let json_report = temp_dir.path().join("report.json");

    // Run the actual CLI binary
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    let args = build_cli_args(
        &sitemap_url,
        Some(csv_report.to_str().unwrap()),
        Some(json_report.to_str().unwrap()),
        None,
        None,
    );
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    // Should succeed even with missing sitemaps (processes 0 URLs)
    assert!(
        output.status.success(),
        "Should succeed even when sitemaps are missing"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show error messages about missing sitemaps
    assert!(
        stderr.contains("ERROR") && stderr.contains("missing"),
        "Should report errors about missing sitemaps in stderr"
    );

    // Report should show 0 requests processed
    assert!(
        stdout.contains("Total Requests Processed") && stdout.contains("0"),
        "Should show 0 requests processed"
    );

    // Verify CSV report was created but contains only header
    assert!(csv_report.exists(), "CSV report should be created");
    let csv_content = fs::read_to_string(&csv_report).expect("Failed to read CSV report");
    assert!(
        csv_content.contains("URL") && csv_content.contains("Response Time"),
        "CSV should have header"
    );
    // CSV should have 1 line (just header, no data)
    assert_eq!(
        csv_content.lines().count(),
        1,
        "CSV should have only header line"
    );

    // Verify JSON report was created with 0 responses
    assert!(json_report.exists(), "JSON report should be created");
    let json_content = fs::read_to_string(&json_report).expect("Failed to read JSON report");
    let json: serde_json::Value =
        serde_json::from_str(&json_content).expect("JSON should be valid");
    assert_eq!(
        json["responses"].as_array().unwrap().len(),
        0,
        "JSON should have 0 responses"
    );
}
