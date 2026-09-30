//! End-to-end tests for the different ways URLs can be handed to siteprobe:
//! a plain-text list from a file, stdin, or a remote URL, and individual
//! URLs passed via `--url`.

use std::io::Write;
use std::process::{Command, Output, Stdio};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Starts a mock server that answers 200 on each of the given paths.
async fn server_with_pages(paths: &[&str]) -> MockServer {
    let server = MockServer::start().await;
    for p in paths {
        Mock::given(method("GET"))
            .and(path(*p))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>ok</html>"))
            .mount(&server)
            .await;
    }
    server
}

/// Runs the siteprobe binary in `--json` mode, optionally feeding it stdin.
fn run_json(args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"])
        .args(args)
        .args(["--json", "--user-agent", "test-agent"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match stdin {
        Some(input) => {
            let mut child = cmd
                .stdin(Stdio::piped())
                .spawn()
                .expect("Failed to spawn siteprobe binary");
            child
                .stdin
                .take()
                .expect("stdin should be piped")
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
            // Dropping the handle above closes stdin so the child sees EOF.
            child
                .wait_with_output()
                .expect("Failed to wait for siteprobe binary")
        }
        // `Command::output` closes stdin, so the child reads EOF immediately.
        None => cmd.output().expect("Failed to execute siteprobe binary"),
    }
}

fn parse_json(output: &Output) -> serde_json::Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout should be JSON: {}\nstdout: {}\nstderr: {}",
            e,
            String::from_utf8_lossy(&output.stdout),
            stderr
        )
    })
}

/// Extracts the probed URLs from the JSON report, sorted.
fn probed_urls(json: &serde_json::Value) -> Vec<String> {
    let mut urls: Vec<String> = json["responses"]
        .as_array()
        .expect("responses should be an array")
        .iter()
        .map(|r| {
            r["url"]
                .as_str()
                .expect("url should be a string")
                .to_string()
        })
        .collect();
    urls.sort();
    urls
}

fn write_temp_file(suffix: &str, content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .prefix("siteprobe_test_")
        .suffix(suffix)
        .tempfile()
        .expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    file
}

// ===========================================================================================
// Plain-text URL lists
// ===========================================================================================

#[tokio::test]
async fn test_file_source_plain_text_list() {
    let server = server_with_pages(&["/", "/about"]).await;
    let base = server.uri();

    let list = format!(
        "# Pages to check\n\n{base}/\n  {base}/about  \nthis is not a url\n",
        base = base
    );
    let file = write_temp_file(".txt", &list);
    let file_path = file.path().to_str().unwrap();

    let output = run_json(&[file_path], None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(
        probed_urls(&json),
        vec![format!("{}/", base), format!("{}/about", base)]
    );
    assert_eq!(json["config"]["sitemapUrl"], file_path);

    // The invalid line is reported on stderr but does not abort the run.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("this is not a url"),
        "stderr should warn about the invalid line: {}",
        stderr
    );
}

#[tokio::test]
async fn test_stdin_source_plain_text_list() {
    let server = server_with_pages(&["/", "/contact"]).await;
    let base = server.uri();

    let list = format!("{base}/\n{base}/contact\n", base = base);
    let output = run_json(&["-"], Some(&list));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(
        probed_urls(&json),
        vec![format!("{}/", base), format!("{}/contact", base)]
    );
    assert_eq!(json["config"]["sitemapUrl"], "URLs from stdin");
}

#[tokio::test]
async fn test_remote_source_plain_text_list() {
    let server = server_with_pages(&["/", "/blog"]).await;
    let base = server.uri();

    let list = format!("{base}/\n{base}/blog\n", base = base);
    Mock::given(method("GET"))
        .and(path("/urls.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list))
        .mount(&server)
        .await;

    let list_url = format!("{}/urls.txt", base);
    let output = run_json(&[&list_url], None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(
        probed_urls(&json),
        vec![format!("{}/", base), format!("{}/blog", base)]
    );
    assert_eq!(json["config"]["sitemapUrl"], list_url);
}

// ===========================================================================================
// Sitemaps from local sources (format detection is independent of the source)
// ===========================================================================================

#[tokio::test]
async fn test_file_source_sitemap_xml() {
    let server = server_with_pages(&["/", "/catalog"]).await;
    let base = server.uri();

    let sitemap_xml =
        include_str!("fixtures/sitemap_valid.xml").replace("http://www.example.com", &base);
    let file = write_temp_file(".xml", &sitemap_xml);

    let output = run_json(&[file.path().to_str().unwrap()], None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    let urls = probed_urls(&json);
    assert_eq!(
        urls.len(),
        5,
        "all <loc> entries should be probed: {:?}",
        urls
    );
    assert!(urls.contains(&format!("{}/", base)));
}

#[tokio::test]
async fn test_stdin_source_sitemap_xml() {
    let server = server_with_pages(&["/", "/catalog"]).await;
    let base = server.uri();

    let sitemap_xml =
        include_str!("fixtures/sitemap_valid.xml").replace("http://www.example.com", &base);

    let output = run_json(&["-"], Some(&sitemap_xml));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(probed_urls(&json).len(), 5);
}

// ===========================================================================================
// --url
// ===========================================================================================

#[tokio::test]
async fn test_url_flag_only() {
    let server = server_with_pages(&["/", "/pricing"]).await;
    let base = server.uri();

    let root = format!("{}/", base);
    let pricing = format!("{}/pricing", base);
    let output = run_json(&["-u", &root, "--url", &pricing], None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(probed_urls(&json), vec![root, pricing]);
    assert_eq!(json["config"]["sitemapUrl"], "URLs from the command line");
}

#[tokio::test]
async fn test_url_flag_combined_with_source_is_merged_and_deduplicated() {
    let server = server_with_pages(&["/", "/about", "/new"]).await;
    let base = server.uri();

    let list = format!("{base}/\n{base}/about\n", base = base);
    let file = write_temp_file(".txt", &list);

    // `/about` appears both in the file and on the command line.
    let about = format!("{}/about", base);
    let new_page = format!("{}/new", base);
    let output = run_json(
        &[file.path().to_str().unwrap(), "-u", &about, "-u", &new_page],
        None,
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = parse_json(&output);
    assert_eq!(
        probed_urls(&json),
        vec![format!("{}/", base), about, new_page]
    );
}

#[test]
fn test_url_flag_rejects_non_http_scheme() {
    let output = run_json(&["-u", "ftp://example.com/file"], None);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("http or https"),
        "stderr should explain the scheme requirement: {}",
        stderr
    );
}

// ===========================================================================================
// Error cases
// ===========================================================================================

#[test]
fn test_source_without_any_urls_fails() {
    let file = write_temp_file(".txt", "# only comments here\n\nnot a url either\n");

    let output = run_json(&[file.path().to_str().unwrap()], None);
    assert!(!output.status.success(), "Should fail without any URLs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not contain any URLs"),
        "stderr should explain that no URLs were found: {}",
        stderr
    );
}

#[tokio::test]
async fn test_remote_source_redirect_fails_with_target_and_hint() {
    let server = MockServer::start().await;
    let base = server.uri();
    let target = format!("{}/real-sitemap.xml", base);

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(
            ResponseTemplate::new(301)
                .insert_header("Location", target.as_str())
                .set_body_string("<html><body>301 Moved Permanently</body></html>"),
        )
        .mount(&server)
        .await;

    let output = run_json(&[&format!("{}/sitemap.xml", base)], None);
    assert!(
        !output.status.success(),
        "Should fail when the source redirects"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("301 Moved Permanently"),
        "stderr should name the redirect status: {}",
        stderr
    );
    assert!(
        stderr.contains(&target),
        "stderr should name the redirect target: {}",
        stderr
    );
    assert!(
        stderr.contains("--follow-redirects"),
        "stderr should mention the flag: {}",
        stderr
    );
    assert!(
        !stderr.contains("does not contain any URLs"),
        "the redirect body must not be reported as an empty source: {}",
        stderr
    );
}

#[tokio::test]
async fn test_remote_source_redirect_resolves_relative_location() {
    let server = MockServer::start().await;
    let base = server.uri();

    Mock::given(method("GET"))
        .and(path("/sitemap.xml"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", "/real-sitemap.xml")
                .set_body_string(""),
        )
        .mount(&server)
        .await;

    let output = run_json(&[&format!("{}/sitemap.xml", base)], None);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("{}/real-sitemap.xml", base)),
        "a relative Location should be resolved against the source URL: {}",
        stderr
    );
}

#[test]
fn test_empty_stdin_fails() {
    let output = run_json(&["-"], Some(""));
    assert!(!output.status.success(), "Should fail on empty stdin");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not contain any URLs"),
        "stderr should explain that no URLs were found: {}",
        stderr
    );
}

#[test]
fn test_missing_file_fails_at_argument_parsing() {
    let output = run_json(&["/definitely/not/here/urls.txt"], None);
    assert!(!output.status.success(), "Should fail for a missing file");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("neither an http(s) URL, an existing file"),
        "stderr should explain what SOURCE accepts: {}",
        stderr
    );
}
