use std::process::Command;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SITEMAP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{BASE}/page1</loc></url>
</urlset>"#;

async fn setup_mock_server() -> MockServer {
    MockServer::start().await
}

fn run_siteprobe(sitemap_url: &str, extra_args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run", "--quiet", "--", sitemap_url,
        "--user-agent", "test-agent",
        "--request-timeout", "10",
        "--concurrency-limit", "1",
        "--json",
    ]);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute siteprobe")
}

#[tokio::test]
async fn test_custom_header_is_sent() {
    let server = setup_mock_server().await;
    let base = server.uri();

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SITEMAP_XML.replace("{BASE}", &base)),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/page1"))
        .and(header("X-Custom-Token", "secret123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let output = run_siteprobe(
        &format!("{}/sitemap.xml", base),
        &["-H", "X-Custom-Token: secret123"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("200"),
        "Expected 200 response when custom header is matched. stdout: {}",
        stdout
    );
}

#[tokio::test]
async fn test_multiple_custom_headers() {
    let server = setup_mock_server().await;
    let base = server.uri();

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SITEMAP_XML.replace("{BASE}", &base)),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/page1"))
        .and(header("X-First", "one"))
        .and(header("X-Second", "two"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let output = run_siteprobe(
        &format!("{}/sitemap.xml", base),
        &["-H", "X-First: one", "-H", "X-Second: two"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("200"),
        "Expected 200 with multiple headers. stdout: {}",
        stdout
    );
}

#[tokio::test]
async fn test_header_overrides_basic_auth() {
    let server = setup_mock_server().await;
    let base = server.uri();

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SITEMAP_XML.replace("{BASE}", &base)),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/page1"))
        .and(header("Authorization", "Bearer mytoken"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let output = run_siteprobe(
        &format!("{}/sitemap.xml", base),
        &[
            "--basic-auth", "user:pass",
            "-H", "Authorization: Bearer mytoken",
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("200"),
        "Expected Bearer header to override basic auth. stdout: {}",
        stdout
    );
}

#[test]
fn test_invalid_header_format_rejected() {
    let output = Command::new("cargo")
        .args([
            "run", "--quiet", "--",
            "https://example.com/sitemap.xml",
            "-H", "NoColonHere",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "Should fail with invalid header format"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing ':'"),
        "Expected error about missing colon. stderr: {}",
        stderr
    );
}

#[test]
fn test_empty_header_name_rejected() {
    let output = Command::new("cargo")
        .args([
            "run", "--quiet", "--",
            "https://example.com/sitemap.xml",
            "-H", ": value",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "Should fail with empty header name"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("name must not be empty"),
        "Expected error about empty name. stderr: {}",
        stderr
    );
}

#[cfg(test)]
mod validate_header_tests {
    use siteprobe::options::validate_header;

    #[test]
    fn test_valid_header() {
        assert!(validate_header("X-Token: abc123").is_ok());
    }

    #[test]
    fn test_header_with_extra_colons() {
        assert!(validate_header("X-Data: foo:bar:baz").is_ok());
    }

    #[test]
    fn test_missing_colon() {
        assert!(validate_header("NoColon").is_err());
    }

    #[test]
    fn test_empty_name() {
        assert!(validate_header(": value").is_err());
    }
}
