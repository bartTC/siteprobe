# Changelog

## v1.4.0 (WIP)

Maintenance release. No new features, but a couple of fixes, internal
modernization, and a new minimum supported Rust version:

- Fixed a bug where URLs that timed out or failed to connect were silently
  missing from the report entirely (the request task crashed internally).
  They now show up with their mapped status code (408/502/400) as intended,
  and any request failure that can't be mapped to a status code is reported
  on stderr instead of being dropped.
- Fixed median, P90, P95, and P99 response time statistics: they were
  computed on the unsorted list of response times, effectively returning
  arbitrary values. Response times are now sorted before the percentiles
  are taken.
- Fixed the "output directory already exists" warning being printed to
  stdout, which corrupted the `--json` output when `--output-dir` pointed
  to an existing directory. It is now printed to stderr.
- Response time values in the text report now show real fractional
  milliseconds instead of always ending in `.00`.
- Detection of explicitly passed CLI arguments (for config file merging) now
  uses clap's value source tracking instead of scanning the raw command line,
  which was fragile around combined short flags like `-c4`.
- Upgraded all dependencies to their latest versions.
- Modernized the codebase: Rust edition 2024, `std::sync::LazyLock` instead
  of the `once_cell` crate, non-blocking file writes via `tokio::fs`, and a
  declared minimum supported Rust version (1.86). The edition 2021 downgrade
  from v1.2.2 no longer served its purpose, since today's dependency tree
  already requires Rust 1.86+ to build from source.
- CI now enforces `cargo fmt` and `cargo clippy -D warnings`.

## v1.3.0 (2026-02-16)

- Added gzip sitemap support. Siteprobe now handles `.xml.gz` sitemaps,
  detecting gzip compression via URL suffix or magic bytes and decompressing
  automatically. Sitemap index files referencing `.xml.gz` entries are also
  supported.
- Added meaningful exit codes for CI/CD integration: `0` for success, `1` if
  any URL returned 4xx/5xx or failed, `2` if any URL exceeded the slow
  threshold (`--slow-threshold`).
- Added `--retries N` option (default: 0) to retry failed requests. Retries
  on network errors or 5xx responses with a 1-second delay between attempts.
- Added `--json` flag to output the JSON report to stdout, suppressing all
  other console output for clean piping into other tools.
- Added `--report-path-html` option to generate a self-contained HTML report
  with summary statistics, response time distribution histogram, status code
  breakdown chart, and a sortable table of all responses.
- Added `-H` / `--header` option to send custom headers with every request.
  Supports any `Name: Value` format and can be repeated for multiple headers.
  Useful for token-based auth, session cookies, API keys, etc. Also supported
  in the `.siteprobe.toml` config file via the `headers` array field.
- Added `.siteprobe.toml` config file support. Options can be set in a TOML
  file (loaded from the current directory by default, or via `--config`).
  CLI arguments take priority over config file values.
- Updated README with all installation methods (`uvx`, `pipx`, Homebrew,
  pip, Cargo).

## v1.2.2 (2026-02-16)

- Downgraded Rust edition from 2024 to 2021 for compatibility with older Rust
  toolchains (e.g., Cargo 1.75 shipped with Ubuntu). Replaced let chains and
  adjusted never-type fallback usage to compile under edition 2021.
- Switched TLS backend from OpenSSL to rustls. This eliminates the runtime
  dependency on system OpenSSL libraries, fixing "libssl not found" errors
  when installing via `uvx`/`pip` on Linux.

## v1.2.1 (2026-01-20)

- Added Homebrew installation support (`brew install bartTC/siteprobe/siteprobe`).
- Added PyPI installation support (`pip install siteprobe` or `pipx install siteprobe`).
- Shortened package description for Homebrew compatibility.

## v1.2.0 (2026-01-01)

- Added tilde (`~`) expansion support for path arguments (`--report-path`,
  `--report-path-json`, `--output-dir`). Previously, using the `=` syntax
  (e.g., `--report-path-json=~/report.json`) would fail because the shell
  doesn't expand `~` in that context.

## v1.1.0 (2025-11-23)

- Fixed a division by zero error when the sitemap contains no URLs or no URLs are processed.
- Fixed table border misalignment in the report by replacing emojis with inconsistent width handling.
- Fixed potential integer overflow in random number generation.
- Fixed type mismatches for `SLOW_NUM` and `request_timeout` options.

## v1.0.0 (2025-09-05)

- This has demonstrated stability and maturity, making it suitable for a v1.0 release.

## v0.5.2 (2025-05-11)

- Fixed an issue where the calculated rate goes under the rate limiter threshold of 1
  per minute.

## v0.5.0 (2025-06-07)

- Enhance the clarity of error messages.
- Introduced a new rate-limiting feature, allowing users to define the rate at which
  sitemap URLs are fetched. E.g.: 60 requests per minute (`-l 60/1m`) or 300 requests
  every 5 minutes (`-l 300/5m`).

## v0.4.0 (2025-05-11)

- An appropriate error message will be displayed for an invalid sitemap URL.

## v0.3.0 (2025-04-27)

- Introduced the `--report-path-json` option to generate a detailed request and
  performance report in JSON format.

## v0.2.0 (2025-03-12)

- The 'slow responses' list is now optional and will only be displayed if the
  `--slow-threshold` option is specified.
- The progress bar now shows the estimated remaining time.
- Fixed an issue where the follow redirect option was not functioning as expected.

## v0.1.0 (2025-03-11)

- Initial release with all core features.