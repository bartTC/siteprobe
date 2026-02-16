use siteprobe::sitemap::{
    decompress_gzip, extract_sitemap_urls, identify_sitemap_type, is_gzip_content, SitemapType,
};

// ===========================================================================================
// identify_sitemap_type Tests
// ===========================================================================================

#[test]
fn test_identify_sitemap_type_urlset() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <loc>http://www.example.com/</loc>
      <lastmod>2005-01-01</lastmod>
      <changefreq>monthly</changefreq>
      <priority>0.8</priority>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=12&amp;desc=vacation_hawaii</loc>
      <changefreq>weekly</changefreq>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=73&amp;desc=vacation_new_zealand</loc>
      <lastmod>2004-12-23</lastmod>
      <changefreq>weekly</changefreq>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=74&amp;desc=vacation_newfoundland</loc>
      <lastmod>2004-12-23T18:00:15+00:00</lastmod>
      <priority>0.3</priority>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=83&amp;desc=vacation_usa</loc>
      <lastmod>2004-11-23</lastmod>
   </url>
</urlset>"#;
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::UrlSet);
}

#[test]
fn test_identify_sitemap_type_sitemapindex() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <sitemap>
      <loc>http://www.example.com/sitemap1.xml</loc>
      <lastmod>2004-10-01T18:23:17+00:00</lastmod>
   </sitemap>
   <sitemap>
      <loc>http://www.example.com/sitemap2.xml</loc>
      <lastmod>2005-01-01</lastmod>
   </sitemap>
   <sitemap>
      <loc>http://www.example.com/sitemap3.xml</loc>
   </sitemap>
</sitemapindex>"#;
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::SitemapIndex);
}

#[test]
fn test_identify_sitemap_type_invalid() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
   <channel>
      <title>Example RSS Feed</title>
      <link>http://www.example.com/</link>
      <description>This is not a sitemap</description>
      <item>
         <title>Example Item</title>
         <link>http://www.example.com/item1</link>
      </item>
   </channel>
</rss>"#;
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

#[test]
fn test_identify_sitemap_type_empty() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#;
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::UrlSet);
}

#[test]
fn test_identify_sitemap_type_malformed() {
    let xml = "This is not XML at all";
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

#[test]
fn test_identify_sitemap_type_empty_string() {
    let xml = "";
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

// ===========================================================================================
// extract_sitemap_urls Tests
// ===========================================================================================

#[test]
fn test_extract_sitemap_urls_valid() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <loc>http://www.example.com/</loc>
      <lastmod>2005-01-01</lastmod>
      <changefreq>monthly</changefreq>
      <priority>0.8</priority>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=12&amp;desc=vacation_hawaii</loc>
      <changefreq>weekly</changefreq>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=73&amp;desc=vacation_new_zealand</loc>
      <lastmod>2004-12-23</lastmod>
      <changefreq>weekly</changefreq>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=74&amp;desc=vacation_newfoundland</loc>
      <lastmod>2004-12-23T18:00:15+00:00</lastmod>
      <priority>0.3</priority>
   </url>
   <url>
      <loc>http://www.example.com/catalog?item=83&amp;desc=vacation_usa</loc>
      <lastmod>2004-11-23</lastmod>
   </url>
</urlset>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 5);
    assert_eq!(urls[0], "http://www.example.com/");
    assert_eq!(
        urls[1],
        "http://www.example.com/catalog?item=12&desc=vacation_hawaii"
    );
    assert_eq!(
        urls[2],
        "http://www.example.com/catalog?item=73&desc=vacation_new_zealand"
    );
    assert_eq!(
        urls[3],
        "http://www.example.com/catalog?item=74&desc=vacation_newfoundland"
    );
    assert_eq!(
        urls[4],
        "http://www.example.com/catalog?item=83&desc=vacation_usa"
    );
}

#[test]
fn test_extract_sitemap_urls_from_index() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <sitemap>
      <loc>http://www.example.com/sitemap1.xml</loc>
      <lastmod>2004-10-01T18:23:17+00:00</lastmod>
   </sitemap>
   <sitemap>
      <loc>http://www.example.com/sitemap2.xml</loc>
      <lastmod>2005-01-01</lastmod>
   </sitemap>
   <sitemap>
      <loc>http://www.example.com/sitemap3.xml</loc>
   </sitemap>
</sitemapindex>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 3);
    assert_eq!(urls[0], "http://www.example.com/sitemap1.xml");
    assert_eq!(urls[1], "http://www.example.com/sitemap2.xml");
    assert_eq!(urls[2], "http://www.example.com/sitemap3.xml");
}

#[test]
fn test_extract_sitemap_urls_with_escapes() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <loc>http://www.example.com/page?id=1&amp;category=test</loc>
      <lastmod>2005-01-01</lastmod>
   </url>
   <url>
      <loc>http://www.example.com/special&lt;chars&gt;</loc>
   </url>
   <url>
      <loc>http://www.example.com/path/with/&quot;quotes&quot;</loc>
   </url>
</urlset>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 3);
    // XML entities should be unescaped
    assert_eq!(urls[0], "http://www.example.com/page?id=1&category=test");
    assert_eq!(urls[1], "http://www.example.com/special<chars>");
    assert_eq!(urls[2], "http://www.example.com/path/with/\"quotes\"");
}

#[test]
fn test_extract_sitemap_urls_empty() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 0);
}

#[test]
fn test_extract_sitemap_urls_malformed() {
    let xml = "This is not XML at all";
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 0);
}

#[test]
fn test_extract_sitemap_urls_empty_string() {
    let xml = "";
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 0);
}

#[test]
fn test_extract_sitemap_urls_no_loc_tags() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <lastmod>2005-01-01</lastmod>
   </url>
</urlset>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 0);
}

#[test]
fn test_extract_sitemap_urls_nested_structure() {
    // Test that it correctly extracts URLs even with nested XML structure
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <loc>http://www.example.com/page1</loc>
      <lastmod>2005-01-01</lastmod>
      <changefreq>monthly</changefreq>
      <priority>0.8</priority>
   </url>
   <url>
      <loc>http://www.example.com/page2</loc>
   </url>
</urlset>"#;
    let urls = extract_sitemap_urls(xml);

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "http://www.example.com/page1");
    assert_eq!(urls[1], "http://www.example.com/page2");
}

// ===========================================================================================
// Edge Cases - Completely Empty Responses
// ===========================================================================================

#[test]
fn test_identify_sitemap_type_whitespace_only() {
    let xml = "   \n\t  \n  ";
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

#[test]
fn test_extract_sitemap_urls_whitespace_only() {
    let xml = "   \n\t  \n  ";
    let urls = extract_sitemap_urls(xml);
    assert_eq!(urls.len(), 0);
}

#[test]
fn test_identify_sitemap_type_null_response() {
    // Simulating a completely empty HTTP response body
    let xml = "";
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

#[test]
fn test_extract_sitemap_urls_null_response() {
    // Simulating a completely empty HTTP response body
    let xml = "";
    let urls = extract_sitemap_urls(xml);
    assert_eq!(urls.len(), 0);
}

#[test]
fn test_identify_sitemap_type_incomplete_xml() {
    // XML declaration only, no actual content
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
    let result = identify_sitemap_type(xml);
    assert_eq!(result, SitemapType::Unknown);
}

#[test]
fn test_extract_sitemap_urls_incomplete_xml() {
    // XML declaration only, no actual content
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
    let urls = extract_sitemap_urls(xml);
    assert_eq!(urls.len(), 0);
}

// ===========================================================================================
// Gzip Support Tests
// ===========================================================================================

#[test]
fn test_is_gzip_content_by_url_suffix() {
    assert!(is_gzip_content("https://example.com/sitemap.xml.gz", &[]));
    assert!(is_gzip_content("https://example.com/sitemap.gz", &[]));
    assert!(!is_gzip_content("https://example.com/sitemap.xml", &[]));
}

#[test]
fn test_is_gzip_content_by_magic_bytes() {
    // Gzip magic bytes: 0x1f, 0x8b
    assert!(is_gzip_content(
        "https://example.com/sitemap.xml",
        &[0x1f, 0x8b, 0x08]
    ));
    assert!(!is_gzip_content(
        "https://example.com/sitemap.xml",
        &[0x3c, 0x3f]
    ));
    assert!(!is_gzip_content("https://example.com/sitemap.xml", &[0x1f]));
    assert!(!is_gzip_content("https://example.com/sitemap.xml", &[]));
}

#[test]
fn test_decompress_gzip_valid() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <url>
      <loc>http://www.example.com/page1</loc>
   </url>
</urlset>"#;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let decompressed = decompress_gzip(&compressed).unwrap();
    assert_eq!(decompressed, xml);

    // Verify the decompressed XML can be parsed
    let sitemap_type = identify_sitemap_type(&decompressed);
    assert_eq!(sitemap_type, SitemapType::UrlSet);

    let urls = extract_sitemap_urls(&decompressed);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0], "http://www.example.com/page1");
}

#[test]
fn test_decompress_gzip_invalid_data() {
    let result = decompress_gzip(&[0x1f, 0x8b, 0x00, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_decompress_gzip_empty() {
    let result = decompress_gzip(&[]);
    assert!(result.is_err());
}

#[test]
fn test_gzip_roundtrip_sitemap_index() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
   <sitemap>
      <loc>http://www.example.com/sitemap1.xml.gz</loc>
   </sitemap>
   <sitemap>
      <loc>http://www.example.com/sitemap2.xml.gz</loc>
   </sitemap>
</sitemapindex>"#;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let decompressed = decompress_gzip(&compressed).unwrap();
    let sitemap_type = identify_sitemap_type(&decompressed);
    assert_eq!(sitemap_type, SitemapType::SitemapIndex);

    let urls = extract_sitemap_urls(&decompressed);
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "http://www.example.com/sitemap1.xml.gz");
    assert_eq!(urls[1], "http://www.example.com/sitemap2.xml.gz");
}
