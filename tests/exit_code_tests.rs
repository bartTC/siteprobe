use reqwest::StatusCode;
use siteprobe::report::{Report, Response};
use std::collections::VecDeque;
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
        sitemap_url: "https://example.com/sitemap.xml".to_string(),
        concurrency_limit: 1,
        rate_limit: None,
        total_time: Duration::from_secs(1),
        responses: VecDeque::from(responses),
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
fn exit_code_2_when_slow_threshold_exceeded() {
    let report = make_report(vec![
        make_response(200, 100),
        make_response(200, 3500), // 3.5 seconds, exceeds 2.0s threshold
    ]);
    assert_eq!(report.exit_code(Some(2.0)), 2u8.into());
}

#[test]
fn exit_code_1_takes_priority_over_exit_code_2() {
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
