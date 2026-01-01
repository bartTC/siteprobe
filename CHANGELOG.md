# Changelog

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