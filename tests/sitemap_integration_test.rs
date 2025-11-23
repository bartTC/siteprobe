use siteprobe::sitemap::get_sitemap_urls;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_sitemap_urls_with_deduplication() {
    // Start a mock HTTP server
    let mock_server = MockServer::start().await;

    // Read fixture content and replace example.com URLs with mock server URL
    let index_xml = include_str!("fixtures/sitemap_index_valid.xml")
        .replace("http://www.example.com", &mock_server.uri());
    let sitemap1_xml = include_str!("fixtures/sitemap1.xml");
    let sitemap2_xml = include_str!("fixtures/sitemap2.xml");
    let sitemap3_xml = include_str!("fixtures/sitemap3.xml");

    // Mock the sitemap index endpoint
    Mock::given(method("GET"))
        .and(path("/sitemap_index.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(index_xml))
        .mount(&mock_server)
        .await;

    // Mock the individual sitemap endpoints
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

    // Create HTTP client
    let client = reqwest::Client::new();

    // Call get_sitemap_urls with the mock server URL
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    let urls = get_sitemap_urls(&sitemap_url, &client)
        .await
        .expect("Failed to get sitemap URLs");

    // Verify deduplication worked
    // sitemap1: page1, page2, shared-page (3 URLs)
    // sitemap2: page3, shared-page, page4 (3 URLs, shared-page is duplicate)
    // sitemap3: page5, page6 (2 URLs)
    // Total unique: 7 URLs (8 - 1 duplicate)
    assert_eq!(urls.len(), 7, "Should have 7 unique URLs after deduplication");

    // Verify URLs are sorted (side effect of deduplication)
    assert!(urls.windows(2).all(|w| w[0] <= w[1]), "URLs should be sorted");

    // Verify the duplicate was removed
    let shared_page_count = urls
        .iter()
        .filter(|url| url.contains("shared-page"))
        .count();
    assert_eq!(
        shared_page_count, 1,
        "shared-page should appear only once after deduplication"
    );

    // Verify all expected unique URLs are present
    assert!(urls.contains(&"http://www.example.com/page1".to_string()));
    assert!(urls.contains(&"http://www.example.com/page2".to_string()));
    assert!(urls.contains(&"http://www.example.com/page3".to_string()));
    assert!(urls.contains(&"http://www.example.com/page4".to_string()));
    assert!(urls.contains(&"http://www.example.com/page5".to_string()));
    assert!(urls.contains(&"http://www.example.com/page6".to_string()));
    assert!(urls.contains(&"http://www.example.com/shared-page".to_string()));
}

#[tokio::test]
async fn test_get_sitemap_urls_with_missing_sitemaps() {
    // Test that when a sitemap index references sitemaps that don't exist,
    // we return 0 URLs (all referenced sitemaps fail to load)
    
    let mock_server = MockServer::start().await;

    // Read fixture and replace example.com URLs with mock server URL
    let index_xml = include_str!("fixtures/sitemap_index_invalid.xml")
        .replace("http://www.example.com", &mock_server.uri());

    // Mock the sitemap index endpoint
    Mock::given(method("GET"))
        .and(path("/sitemap_index.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(index_xml))
        .mount(&mock_server)
        .await;

    // Don't mock the missing sitemap endpoints - they will return 404

    // Create HTTP client
    let client = reqwest::Client::new();

    // Call get_sitemap_urls with the mock server URL
    let sitemap_url = format!("{}/sitemap_index.xml", mock_server.uri());
    let urls = get_sitemap_urls(&sitemap_url, &client)
        .await
        .expect("Should succeed even when referenced sitemaps are missing");

    // Should return 0 URLs since all referenced sitemaps failed to load
    assert_eq!(
        urls.len(),
        0,
        "Should have 0 URLs when all referenced sitemaps are missing"
    );
}
