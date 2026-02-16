use std::fs;
use std::process::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("siteprobe_json_test_{}_", prefix))
        .tempdir()
        .expect("Failed to create temp dir")
}

/// Build CLI args with --json flag and optional extras.
fn build_json_cli_args(sitemap_url: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--".to_string(),
        sitemap_url.to_string(),
        "--json".to_string(),
        "--user-agent".to_string(),
        "test-agent".to_string(),
        "--request-timeout".to_string(),
        "10".to_string(),
        "--concurrency-limit".to_string(),
        "5".to_string(),
        "--rate-limit".to_string(),
        "600/1m".to_string(),
        "--append-timestamp".to_string(),
    ]
}

/// Sets up a mock server with a simple sitemap and two page endpoints.
/// Returns the mock server (caller must hold it alive).
async fn setup_mock_server() -> MockServer {
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

    mock_server
}

#[tokio::test]
async fn test_json_flag_produces_valid_json_on_stdout() {
    let mock_server = setup_mock_server().await;
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_json_cli_args(&sitemap_url);

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        output.status.success(),
        "Command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must parse as valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // Verify top-level structure
    assert!(
        json.get("config").is_some(),
        "JSON should have 'config' key"
    );
    assert!(
        json.get("statistics").is_some(),
        "JSON should have 'statistics' key"
    );
    assert!(
        json.get("responses").is_some(),
        "JSON should have 'responses' key"
    );

    // Verify responses array has entries
    let responses = json["responses"]
        .as_array()
        .expect("responses should be an array");
    assert!(!responses.is_empty(), "responses array should not be empty");

    // Verify each response has the expected fields
    for resp in responses {
        assert!(resp.get("url").is_some(), "response should have 'url'");
        assert!(
            resp.get("statusCode").is_some(),
            "response should have 'statusCode'"
        );
        assert!(
            resp.get("responseTime").is_some(),
            "response should have 'responseTime'"
        );
        assert!(
            resp.get("responseSize").is_some(),
            "response should have 'responseSize'"
        );

        // statusCode should be a number
        assert!(resp["statusCode"].is_u64(), "statusCode should be a number");
    }
}

#[tokio::test]
async fn test_json_flag_suppresses_table_output() {
    let mock_server = setup_mock_server().await;
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_json_cli_args(&sitemap_url);

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT contain text-mode report markers
    assert!(
        !stdout.contains("Statistics for"),
        "stdout should not contain 'Statistics for' when --json is used"
    );
    assert!(
        !stdout.contains("Total Requests Processed"),
        "stdout should not contain table text when --json is used"
    );
    assert!(
        !stdout.contains("Success Rate"),
        "stdout should not contain 'Success Rate' text label when --json is used"
    );
    assert!(
        !stdout.contains("Response Time and Performance Statistics:"),
        "stdout should not contain table header when --json is used"
    );
}

#[tokio::test]
async fn test_json_flag_combined_with_report_path_json() {
    let mock_server = setup_mock_server().await;
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());

    let temp_dir = temp_dir("json_both");
    let json_report_path = temp_dir.path().join("report.json");

    let mut args = build_json_cli_args(&sitemap_url);
    args.push("--report-path-json".to_string());
    args.push(json_report_path.to_str().unwrap().to_string());

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        output.status.success(),
        "Command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // stdout should be valid JSON
    let stdout_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // File should also exist and be valid JSON
    assert!(
        json_report_path.exists(),
        "JSON report file should be created"
    );
    let file_content = fs::read_to_string(&json_report_path).expect("Failed to read JSON report");
    let file_json: serde_json::Value =
        serde_json::from_str(&file_content).expect("file JSON should be valid");

    // Both should have the same structure
    assert_eq!(
        stdout_json["responses"].as_array().unwrap().len(),
        file_json["responses"].as_array().unwrap().len(),
        "stdout and file should have the same number of responses"
    );

    // Both should have config, statistics, responses
    for key in &["config", "statistics", "responses"] {
        assert!(
            stdout_json.get(*key).is_some(),
            "stdout JSON missing '{}'",
            key
        );
        assert!(file_json.get(*key).is_some(), "file JSON missing '{}'", key);
    }
}

#[tokio::test]
async fn test_json_output_structure_fields() {
    let mock_server = setup_mock_server().await;
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    let args = build_json_cli_args(&sitemap_url);

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    // Verify config fields
    let config = &json["config"];
    assert!(
        config["sitemapUrl"].is_string(),
        "config.sitemapUrl should be a string"
    );
    assert!(
        config["concurrencyLimit"].is_u64(),
        "config.concurrencyLimit should be a number"
    );
    assert!(
        config["elapsedTime"].is_u64(),
        "config.elapsedTime should be a number"
    );
    assert!(
        config["bypassCaching"].is_boolean(),
        "config.bypassCaching should be a boolean"
    );

    // Verify statistics sub-objects exist
    let stats = &json["statistics"];
    assert!(
        stats.get("performance").is_some(),
        "statistics should have 'performance'"
    );
    assert!(
        stats.get("responseTime").is_some(),
        "statistics should have 'responseTime'"
    );
    assert!(
        stats.get("statusCode").is_some(),
        "statistics should have 'statusCode'"
    );

    // Verify responses array entries have correct types
    let responses = json["responses"].as_array().unwrap();
    for resp in responses {
        let url = resp["url"].as_str().unwrap();
        assert!(!url.is_empty(), "url should not be empty");

        let status = resp["statusCode"].as_u64().unwrap();
        assert_eq!(status, 200, "all mocked pages should return 200");

        let time = resp["responseTime"].as_u64();
        assert!(time.is_some(), "responseTime should be a number");

        let size = resp["responseSize"].as_u64();
        assert!(size.is_some(), "responseSize should be a number");
    }
}
