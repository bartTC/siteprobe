# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.5.0] - 2026-09-30

### Added

- Plain-text URL lists as an alternative to `sitemap.xml`: one URL per line,
  blank lines and lines starting with `#` are ignored. The format is detected
  automatically, so any source may be a sitemap, a sitemap index, or a plain
  list. Lines that are not http(s) URLs are skipped with a warning.
- Local files and stdin as URL sources. The positional `SOURCE` argument
  accepts a local file path or `-` to read from stdin, in addition to an
  http(s) URL. This works for sitemaps and URL lists alike, e.g.
  `curl -s https://example.com/sitemap.xml | siteprobe -`.
- `-u/--url <URL>` to probe individual URLs directly, without loading a
  sitemap or list. It can be repeated and combined with `SOURCE`; the URLs
  are merged and deduplicated. `SOURCE` is optional when `--url` is given.
- `--exit-zero` (config file: `exit_zero = true`) to always exit with status
  code 0 after a completed run, even if URLs failed or exceeded the slow
  threshold. Fatal errors, such as an unreadable source, and invalid
  arguments still exit non-zero.

### Changed

- **Breaking:** the exit code for slow responses (`--slow-threshold`) changed
  from `2` to `3`. Exit code `2` was already used for invalid command line
  arguments, so scripts could not tell the two cases apart. Update any CI
  checks that test for `2`.
- The report headline and the `sitemapUrl` field in the JSON report show
  where the URLs came from: the URL or file path, `URLs from stdin`, or
  `URLs from the command line`. The JSON key itself is unchanged.
- Status output names the detected format in words (`sitemap index`,
  `sitemap`, `URL list`) instead of the internal enum variant name.

## [1.4.0] - 2026-09-01

### Changed

- Response time values in the text report show real fractional milliseconds
  instead of always ending in `.00`.
- Detection of explicitly passed CLI arguments (for config file merging)
  relies on the argument parser's own tracking instead of scanning the raw
  command line, which was fragile around combined short flags like `-c4`.
- Upgraded all dependencies to their latest versions, including the major
  bumps reqwest 0.13, quick-xml 0.42, rand 0.10, indicatif 0.18, base64 0.23,
  and toml 1.1, with the required API migrations.
- Modernized the codebase: Rust edition 2024, `std::sync::LazyLock` instead
  of the `once_cell` crate, non-blocking file writes via `tokio::fs`, and a
  declared minimum supported Rust version (1.86). The edition 2021 downgrade
  from v1.2.2 no longer served its purpose, since today's dependency tree
  already requires Rust 1.86+ to build from source.
- CI enforces `cargo fmt` and `cargo clippy -D warnings`.

### Fixed

- URLs that timed out or failed to connect were silently missing from the
  report entirely (the request task crashed internally). They now show up
  with their mapped status code (408/502/400) as intended, and any request
  failure that can't be mapped to a status code is reported on stderr
  instead of being dropped.
- Median, P90, P95, and P99 response time statistics were computed on the
  unsorted list of response times, effectively returning arbitrary values.
  Response times are now sorted before the percentiles are taken.
- The "output directory already exists" warning was printed to stdout, which
  corrupted the `--json` output when `--output-dir` pointed to an existing
  directory. It is now printed to stderr.

## [1.3.0] - 2026-02-16

### Added

- Gzip sitemap support. Siteprobe handles `.xml.gz` sitemaps, detecting gzip
  compression via URL suffix or magic bytes and decompressing automatically.
  Sitemap index files referencing `.xml.gz` entries are also supported.
- Meaningful exit codes for CI/CD integration: `0` for success, `1` if any
  URL returned 4xx/5xx or failed, `2` if any URL exceeded the slow threshold
  (`--slow-threshold`).
- `--retries N` option (default: 0) to retry failed requests. Retries on
  network errors or 5xx responses with a 1-second delay between attempts.
- `--json` flag to output the JSON report to stdout, suppressing all other
  console output for clean piping into other tools.
- `--report-path-html` option to generate a self-contained HTML report with
  summary statistics, response time distribution histogram, status code
  breakdown chart, and a sortable table of all responses.
- `-H` / `--header` option to send custom headers with every request.
  Supports any `Name: Value` format and can be repeated for multiple headers.
  Useful for token-based auth, session cookies, API keys, etc. Also supported
  in the `.siteprobe.toml` config file via the `headers` array field.
- `.siteprobe.toml` config file support. Options can be set in a TOML file
  (loaded from the current directory by default, or via `--config`). CLI
  arguments take priority over config file values.

### Changed

- Updated README with all installation methods (`uvx`, `pipx`, Homebrew,
  pip, Cargo).

## [1.2.2] - 2026-02-16

### Changed

- Downgraded Rust edition from 2024 to 2021 for compatibility with older Rust
  toolchains (e.g., Cargo 1.75 shipped with Ubuntu). Replaced let chains and
  adjusted never-type fallback usage to compile under edition 2021.
- Switched TLS backend from OpenSSL to rustls. This eliminates the runtime
  dependency on system OpenSSL libraries, fixing "libssl not found" errors
  when installing via `uvx`/`pip` on Linux.

## [1.2.1] - 2026-01-20

### Added

- Homebrew installation support (`brew install bartTC/siteprobe/siteprobe`).
- PyPI installation support (`pip install siteprobe` or `pipx install siteprobe`).

### Changed

- Shortened package description for Homebrew compatibility.

## [1.2.0] - 2026-01-01

### Added

- Tilde (`~`) expansion support for path arguments (`--report-path`,
  `--report-path-json`, `--output-dir`). Previously, using the `=` syntax
  (e.g., `--report-path-json=~/report.json`) would fail because the shell
  doesn't expand `~` in that context.

## [1.1.0] - 2025-11-23

### Fixed

- A division by zero error when the sitemap contains no URLs or no URLs are
  processed.
- Table border misalignment in the report by replacing emojis with
  inconsistent width handling.
- Potential integer overflow in random number generation.
- Type mismatches for `SLOW_NUM` and `request_timeout` options.

## [1.0.0] - 2025-09-05

First stable release. No functional changes; the tool has demonstrated
stability and maturity, making it suitable for a v1.0 release.

## [0.5.2] - 2025-06-07

### Fixed

- An issue where the calculated rate goes under the rate limiter threshold of
  1 per minute.

## [0.5.0] - 2025-06-07

### Added

- Rate limiting, allowing users to define the rate at which sitemap URLs are
  fetched. E.g.: 60 requests per minute (`-l 60/1m`) or 300 requests every
  5 minutes (`-l 300/5m`).

### Changed

- Enhanced the clarity of error messages.

## [0.4.0] - 2025-05-11

### Added

- An appropriate error message is displayed for an invalid sitemap URL.

## [0.3.0] - 2025-04-27

### Added

- `--report-path-json` option to generate a detailed request and performance
  report in JSON format.

## [0.2.0] - 2025-03-12

### Added

- The progress bar shows the estimated remaining time.

### Changed

- The 'slow responses' list is optional and only displayed if the
  `--slow-threshold` option is specified.

### Fixed

- The follow redirect option was not functioning as expected.

## [0.1.0] - 2025-03-11

### Added

- Initial release with all core features.

[unreleased]: https://github.com/bartTC/siteprobe/compare/v1.5.0...HEAD
[1.5.0]: https://github.com/bartTC/siteprobe/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/bartTC/siteprobe/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/bartTC/siteprobe/compare/v1.2.2...v1.3.0
[1.2.2]: https://github.com/bartTC/siteprobe/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/bartTC/siteprobe/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/bartTC/siteprobe/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/bartTC/siteprobe/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/bartTC/siteprobe/compare/v0.4.0...v1.0.0
[0.5.2]: https://crates.io/crates/siteprobe/0.5.2
[0.5.0]: https://crates.io/crates/siteprobe/0.5.0
[0.4.0]: https://github.com/bartTC/siteprobe/compare/v0.2.0...v0.4.0
[0.3.0]: https://crates.io/crates/siteprobe/0.3.0
[0.2.0]: https://github.com/bartTC/siteprobe/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/bartTC/siteprobe/releases/tag/v0.1.0
