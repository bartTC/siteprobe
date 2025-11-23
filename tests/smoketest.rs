use siteprobe::sitemap::{fetch_and_generate_report, get_sitemap_urls};
use siteprobe::options::Cli;
use std::sync::Arc;
use tokio::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_end_to_end_sitemap_processing() {
    // End-to-end smoke test: sitemap index → fetch sitemaps → extract URLs → fetch pages → generate report
    
    let mock_server = MockServer::start().await;

    // Setup sitemap index - replace example.com with mock server URL
    let index_xml = include_str!("fixtures/sitemap_index_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());
    
    // Setup individual sitemaps - also replace URLs
    let sitemap1_xml = include_str!("fixtures/sitemap1.xml")
        .replace("http://www.example.com", &mock_server.uri());
    let sitemap2_xml = include_str!("fixtures/sitemap2.xml")
        .replace("http://www.example.com", &mock_server.uri());
    let sitemap3_xml = include_str!("fixtures/sitemap3.xml")
        .replace("http://www.example.com", &mock_server.uri());

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
        "/page1", "/page2", "/page3", "/page4", "/page5", "/page6", "/shared-page"
    ];
    
    for page in pages {
        Mock::given(method("GET"))
            .and(path(page))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!("<html><body>Content for {}</body></html>", page)))
            .mount(&mock_server)
            .await;
    }

    // Create HTTP client
    let client = Arc::new(reqwest::Client::new());

    // Get URLs from sitemap
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    let urls = get_sitemap_urls(&sitemap_url, &client)
        .await
        .expect("Failed to get sitemap URLs");

    // Verify we got 7 unique URLs (deduplication worked)
    assert_eq!(urls.len(), 7, "Should have 7 unique URLs after deduplication");

    // Create minimal CLI options for testing
    let options = Cli {
        sitemap_url: sitemap_url.parse().expect("Invalid URL"),
        user_agent: "test-agent".to_string(),
        request_timeout: 10,
        follow_redirects: false,
        basic_auth: None,
        rate_limit: None,
        concurrency_limit: 5,
        slow_threshold: Some(100.0),
        slow_num: 10,
        report_path: None,
        report_path_json: None,
        output_dir: None,
        append_timestamp: false,
    };

    // Fetch all URLs and generate report
    let start_time = Instant::now();
    let report = fetch_and_generate_report(urls, &client, &options, &start_time)
        .await
        .expect("Failed to generate report");

    // Verify report contains all 7 responses
    assert_eq!(report.responses.len(), 7, "Report should contain 7 responses");

    // Verify all responses were successful (200 OK)
    let success_count = report.responses.iter()
        .filter(|r| r.status_code.is_success())
        .count();
    assert_eq!(success_count, 7, "All 7 responses should be successful");

    // Verify report has valid statistics
    assert!(report.total_time.as_millis() > 0, "Total time should be > 0");
    assert_eq!(report.sitemap_url, sitemap_url, "Sitemap URL should match");
}

#[tokio::test]
async fn test_smoketest_invalid_sitemap_index() {
    // Smoke test: sitemap index with all missing sitemaps should return empty report
    
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

    let client = Arc::new(reqwest::Client::new());
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    
    let urls = get_sitemap_urls(&sitemap_url, &client)
        .await
        .expect("Should succeed even with missing sitemaps");

    // Should get 0 URLs since all referenced sitemaps are missing
    assert_eq!(urls.len(), 0, "Should have 0 URLs when all sitemaps are missing");

    // Create options
    let options = Cli {
        sitemap_url: sitemap_url.parse().expect("Invalid URL"),
        user_agent: "test-agent".to_string(),
        request_timeout: 10,
        follow_redirects: false,
        basic_auth: None,
        rate_limit: None,
        concurrency_limit: 5,
        slow_threshold: Some(100.0),
        slow_num: 10,
        report_path: None,
        report_path_json: None,
        output_dir: None,
        append_timestamp: false,
    };

    // Generate report with empty URL list
    let start_time = Instant::now();
    let report = fetch_and_generate_report(urls, &client, &options, &start_time)
        .await
        .expect("Should succeed with empty URL list");

    // Verify report is empty
    assert_eq!(report.responses.len(), 0, "Report should have 0 responses");
}

#[tokio::test]
async fn test_smoketest_invalid_sitemap_file() {
    // Smoke test: invalid sitemap (RSS feed) should return error
    
    let mock_server = MockServer::start().await;

    // Use invalid sitemap (RSS feed)
    let invalid_xml = include_str!("fixtures/sitemap_invalid.xml");

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(invalid_xml))
        .mount(&mock_server)
        .await;

    let client = Arc::new(reqwest::Client::new());
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    
    // Should return error because sitemap type is Unknown
    let result = get_sitemap_urls(&sitemap_url, &client).await;
    
    assert!(result.is_err(), "Should fail with invalid sitemap");
    assert!(result.unwrap_err().to_string().contains("does not contain any URLs"), 
            "Error should mention no URLs found");
}

#[tokio::test]
async fn test_smoketest_valid_single_sitemap() {
    // Smoke test: single valid sitemap file (not an index)
    
    let mock_server = MockServer::start().await;

    // Use valid single sitemap
    let sitemap_xml = include_str!("fixtures/sitemap_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sitemap_xml))
        .mount(&mock_server)
        .await;

    // Mock the page URLs from the sitemap (including query parameters)
    let pages = vec![
        "/",
        "/catalog",  // Will match both catalog URLs with different query params
    ];
    
    for page in pages {
        Mock::given(method("GET"))
            .and(path(page))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!("<html><body>Page: {}</body></html>", page)))
            .mount(&mock_server)
            .await;
    }

    let client = Arc::new(reqwest::Client::new());
    let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
    
    let urls = get_sitemap_urls(&sitemap_url, &client)
        .await
        .expect("Should succeed with valid sitemap");

    // Should get 5 URLs from the sitemap
    assert_eq!(urls.len(), 5, "Should have 5 URLs from valid sitemap");

    let options = Cli {
        sitemap_url: sitemap_url.parse().expect("Invalid URL"),
        user_agent: "test-agent".to_string(),
        request_timeout: 10,
        follow_redirects: false,
        basic_auth: None,
        rate_limit: None,
        concurrency_limit: 5,
        slow_threshold: Some(100.0),
        slow_num: 10,
        report_path: None,
        report_path_json: None,
        output_dir: None,
        append_timestamp: false,
    };

    let start_time = Instant::now();
    let report = fetch_and_generate_report(urls, &client, &options, &start_time)
        .await
        .expect("Should generate report successfully");

    // Verify all 5 pages were fetched
    assert_eq!(report.responses.len(), 5, "Report should have 5 responses");
    
    let success_count = report.responses.iter()
        .filter(|r| r.status_code.is_success())
        .count();
    assert_eq!(success_count, 5, "All 5 responses should be successful");
}
