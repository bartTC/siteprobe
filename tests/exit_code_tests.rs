use reqwest::StatusCode;
use siteprobe::report::{Report, Response};
use std::time::Duration;

fn make_response(status: u16, response_time_ms: u64) -> Response {
    Response {
        url: format!("https://example.com/{}", status),
        response_time: Duration::from_millis(response_time_ms),
        response_size: 1024,
        status_code: StatusCode::from_u16(status).unwrap(),
    }
}

fn make_report(responses: Vec<Response>) -> Report {
    Report {
        source: "https://example.com/sitemap.xml".to_string(),
        concurrency_limit: 1,
        rate_limit: None,
        total_time: Duration::from_secs(1),
        responses,
    }
}

#[test]
fn exit_code_0_when_all_2xx() {
    let report = make_report(vec![
        make_response(200, 100),
        make_response(201, 150),
        make_response(204, 50),
    ]);
    assert_eq!(report.exit_code(None), 0u8.into());
}

#[test]
fn exit_code_1_when_any_4xx() {
    let report = make_report(vec![make_response(200, 100), make_response(404, 200)]);
    assert_eq!(report.exit_code(None), 1u8.into());
}

#[test]
fn exit_code_1_when_any_5xx() {
    let report = make_report(vec![make_response(200, 100), make_response(500, 200)]);
    assert_eq!(report.exit_code(None), 1u8.into());
}

#[test]
fn exit_code_3_when_slow_threshold_exceeded() {
    let report = make_report(vec![
        make_response(200, 100),
        make_response(200, 3500), // 3.5 seconds, exceeds 2.0s threshold
    ]);
    assert_eq!(report.exit_code(Some(2.0)), 3u8.into());
}

#[test]
fn exit_code_1_takes_priority_over_exit_code_3() {
    let report = make_report(vec![
        make_response(500, 5000), // error AND slow
        make_response(200, 3500), // slow
    ]);
    // Even though there are slow responses, error (exit code 1) takes priority
    assert_eq!(report.exit_code(Some(2.0)), 1u8.into());
}

#[test]
fn exit_code_0_when_slow_threshold_is_none() {
    let report = make_report(vec![
        make_response(200, 10000), // very slow, but no threshold set
        make_response(200, 5000),
    ]);
    assert_eq!(report.exit_code(None), 0u8.into());
}

// ===========================================================================================
// Binary-level exit codes, with and without --exit-zero
// ===========================================================================================

use std::io::Write;
use std::process::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn run_siteprobe(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .args(["--json", "--user-agent", "test-agent"])
        .output()
        .expect("Failed to execute siteprobe binary")
}

fn exit_code(output: &std::process::Output) -> i32 {
    output.status.code().unwrap_or_else(|| {
        panic!(
            "process was terminated by a signal; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[tokio::test]
async fn binary_exits_1_on_404_and_0_with_exit_zero() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let root = format!("{}/", server.uri());
    let missing = format!("{}/missing", server.uri());

    let output = run_siteprobe(&["-u", &root, "-u", &missing]);
    assert_eq!(exit_code(&output), 1);

    let output = run_siteprobe(&["-u", &root, "-u", &missing, "--exit-zero"]);
    assert_eq!(exit_code(&output), 0);
}

#[tokio::test]
async fn binary_exits_3_when_slow_and_0_with_exit_zero() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(300)))
        .mount(&server)
        .await;

    let root = format!("{}/", server.uri());

    let output = run_siteprobe(&["-u", &root, "--slow-threshold", "0.05"]);
    assert_eq!(exit_code(&output), 3);

    let output = run_siteprobe(&["-u", &root, "--slow-threshold", "0.05", "--exit-zero"]);
    assert_eq!(exit_code(&output), 0);
}

#[test]
fn binary_exits_2_on_invalid_arguments() {
    let output = run_siteprobe(&["-u", "https://example.com/", "--no-such-flag"]);
    assert_eq!(exit_code(&output), 2);

    // --exit-zero does not apply to usage errors either.
    let output = run_siteprobe(&[
        "-u",
        "https://example.com/",
        "--no-such-flag",
        "--exit-zero",
    ]);
    assert_eq!(exit_code(&output), 2);
}

#[test]
fn exit_zero_does_not_mask_fatal_errors() {
    // A source without any URLs aborts the run before any report exists.
    let mut file = tempfile::Builder::new()
        .prefix("siteprobe_exit_zero_")
        .suffix(".txt")
        .tempfile()
        .expect("Failed to create temp file");
    file.write_all(b"# nothing to see here\n")
        .expect("Failed to write temp file");

    let output = run_siteprobe(&[file.path().to_str().unwrap(), "--exit-zero"]);
    assert_eq!(exit_code(&output), 1);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not contain any URLs"),
        "stderr should report the fatal error: {}",
        stderr
    );
}
