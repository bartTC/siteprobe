use std::process::Command;

#[test]
fn test_cli_help() {
    // Test --help flag
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--help"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify help output contains expected sections
    assert!(stdout.contains("Usage:"), "Help should show usage");
    assert!(stdout.contains("Arguments:"), "Help should show arguments");
    assert!(stdout.contains("Options:"), "Help should show options");
    assert!(
        stdout.contains("SITEMAP_URL"),
        "Help should mention sitemap URL"
    );

    // Verify key options are documented
    assert!(
        stdout.contains("--user-agent"),
        "Help should document --user-agent"
    );
    assert!(
        stdout.contains("--concurrency-limit"),
        "Help should document --concurrency-limit"
    );
    assert!(
        stdout.contains("--rate-limit"),
        "Help should document --rate-limit"
    );
    assert!(
        stdout.contains("--report-path"),
        "Help should document --report-path"
    );
}

#[test]
fn test_cli_help_short() {
    // Test -h flag
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "-h"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "Help should show usage");
}

#[test]
fn test_cli_version() {
    // Test --version flag
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--version"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success(), "Version command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show version number
    assert!(
        stdout.contains("siteprobe"),
        "Version should mention package name"
    );
    // Version should be in format X.Y.Z
    assert!(
        stdout.chars().any(|c| c.is_numeric()),
        "Version should contain numbers"
    );
}

#[test]
fn test_cli_version_short() {
    // Test -V flag
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "-V"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(output.status.success(), "Version command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("siteprobe"),
        "Version should mention package name"
    );
}

#[test]
fn test_cli_missing_required_argument() {
    // Test running without required sitemap URL
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        !output.status.success(),
        "Should fail without required argument"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention the missing argument
    assert!(
        stderr.contains("required") || stderr.contains("SITEMAP_URL"),
        "Error should mention required argument"
    );
}

#[test]
fn test_cli_invalid_url() {
    // Test with invalid URL format
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "not-a-valid-url"])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(!output.status.success(), "Should fail with invalid URL");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention URL or parsing error
    assert!(
        stderr.contains("invalid") || stderr.contains("URL") || stderr.contains("parse"),
        "Error should mention URL problem: {}",
        stderr
    );
}

#[test]
fn test_cli_invalid_option() {
    // Test with unknown option
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--unknown-option",
            "http://example.com/sitemap.xml",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(!output.status.success(), "Should fail with unknown option");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention the unknown option
    assert!(
        stderr.contains("unknown")
            || stderr.contains("unexpected")
            || stderr.contains("unrecognized"),
        "Error should mention unknown option"
    );
}

#[test]
fn test_cli_invalid_concurrency_limit() {
    // Test with invalid concurrency limit (non-numeric)
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--concurrency-limit",
            "not-a-number",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        !output.status.success(),
        "Should fail with invalid concurrency limit"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("parse"),
        "Error should mention invalid value"
    );
}

#[test]
fn test_cli_invalid_rate_limit_format() {
    // Test with invalid rate limit format
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--rate-limit",
            "invalid-format",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        !output.status.success(),
        "Should fail with invalid rate limit format"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("format") || stderr.contains("invalid") || stderr.contains("parse"),
        "Error should mention format problem"
    );
}

#[test]
fn test_cli_invalid_basic_auth_format() {
    // Test with invalid basic auth format (missing colon)
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--basic-auth",
            "invalid-no-colon",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    assert!(
        !output.status.success(),
        "Should fail with invalid basic auth format"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("format")
            || stderr.contains("invalid")
            || stderr.contains("username:password"),
        "Error should mention format requirement"
    );
}

#[test]
fn test_cli_valid_flags_combination() {
    // Test that valid flag combinations are accepted (even if they fail later due to network)
    // We're just testing the CLI parsing here, not the actual execution
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--user-agent",
            "test-bot",
            "--request-timeout",
            "30",
            "--concurrency-limit",
            "10",
            "--follow-redirects",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    // This will fail because the URL doesn't exist, but it should fail AFTER parsing
    // The important thing is that the arguments are accepted
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain argument parsing errors
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "Should not have argument parsing errors for valid flags"
    );
}

#[test]
fn test_cli_rate_limit_valid_formats() {
    // Test various valid rate limit formats
    let formats = vec!["100/1s", "50/5m", "1000/1h", "10/10s"];

    for format in formats {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--",
                "http://example.com/sitemap.xml",
                "--rate-limit",
                format,
            ])
            .output()
            .expect("Failed to execute siteprobe binary");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should not fail due to rate limit parsing
        assert!(
            !stderr.contains("Invalid rate limit format"),
            "Rate limit format '{}' should be valid",
            format
        );
    }
}

#[test]
fn test_cli_multiple_report_paths() {
    // Test that both CSV and JSON report paths can be specified together
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--report-path",
            "/tmp/report.csv",
            "--report-path-json",
            "/tmp/report.json",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have argument parsing errors
    assert!(
        !stderr.contains("unexpected") && !stderr.contains("cannot be used"),
        "Should accept both report paths together"
    );
}

#[test]
fn test_cli_append_timestamp_flag() {
    // Test the append-timestamp flag
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--append-timestamp",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have argument parsing errors
    assert!(
        !stderr.contains("unrecognized") && !stderr.contains("unexpected"),
        "Should accept --append-timestamp flag"
    );
}

#[test]
fn test_cli_output_dir_option() {
    // Test the output-dir option
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            "--output-dir",
            "/tmp/pages",
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have argument parsing errors
    assert!(
        !stderr.contains("unrecognized") && !stderr.contains("unexpected"),
        "Should accept --output-dir option"
    );
}

#[test]
fn test_cli_tilde_expansion_report_path_json() {
    // Test that tilde is expanded correctly in --report-path-json
    // Using = syntax which previously didn't expand tilde
    let home = std::env::var("HOME").expect("HOME not set");
    let test_dir_name = format!("siteprobe_test_{}", std::process::id());
    let test_path = format!("{}/{}", home, test_dir_name);

    // Clean up in case of previous failed run
    let _ = std::fs::remove_dir_all(&test_path);
    std::fs::create_dir_all(&test_path).expect("Failed to create test dir");

    let report_path = format!("~/{}/report.json", test_dir_name);

    // Run siteprobe with tilde path using = syntax
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            &format!("--report-path-json={}", report_path),
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail due to path parsing
    assert!(
        !stderr.contains("No such file or directory") || !stderr.contains(&report_path),
        "Tilde should be expanded, not treated as literal path"
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&test_path);
}

#[test]
fn test_cli_tilde_expansion_report_path_csv() {
    // Test that tilde is expanded correctly in --report-path
    let home = std::env::var("HOME").expect("HOME not set");
    let test_dir_name = format!("siteprobe_test_{}", std::process::id());
    let test_path = format!("{}/{}", home, test_dir_name);

    // Clean up in case of previous failed run
    let _ = std::fs::remove_dir_all(&test_path);
    std::fs::create_dir_all(&test_path).expect("Failed to create test dir");

    let report_path = format!("~/{}/report.csv", test_dir_name);

    // Run siteprobe with tilde path using = syntax
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            &format!("--report-path={}", report_path),
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail due to path parsing (the ~ literal path error)
    assert!(
        !stderr.contains("No such file or directory") || !stderr.contains(&report_path),
        "Tilde should be expanded, not treated as literal path"
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&test_path);
}

#[test]
fn test_cli_tilde_expansion_output_dir() {
    // Test that tilde is expanded correctly in --output-dir
    let home = std::env::var("HOME").expect("HOME not set");
    let test_dir_name = format!("siteprobe_test_output_{}", std::process::id());
    let test_path = format!("{}/{}", home, test_dir_name);

    // Clean up in case of previous failed run
    let _ = std::fs::remove_dir_all(&test_path);

    let output_dir = format!("~/{}", test_dir_name);

    // Run siteprobe with tilde path using = syntax
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "http://example.com/sitemap.xml",
            &format!("--output-dir={}", output_dir),
        ])
        .output()
        .expect("Failed to execute siteprobe binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail with "Failed to create directory" for the tilde path
    assert!(
        !stderr.contains("Failed to create directory"),
        "Tilde should be expanded, directory creation should work"
    );

    // The directory should have been created in the home folder
    assert!(
        std::path::Path::new(&test_path).exists(),
        "Directory should be created at expanded path: {}",
        test_path
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&test_path);
}
