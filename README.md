# Siteprobe

Siteprobe is a command-line tool, written in Rust, that fetches every URL from a
`sitemap.xml` or a plain list of URLs, checks that each one exists, and reports
how long it took to respond. It prints statistics to the terminal, can write
CSV, JSON, and HTML reports, and its exit code reflects the result, so it can
gate a CI pipeline.

![Screenshot of Siteprobe statistics](https://github.com/bartTC/siteprobe/blob/main/docs/screenshot.png?raw=true)

Website: <https://barttc.github.io/siteprobe/>

## Installation

```sh
# Homebrew (macOS/Linux)
brew install bartTC/siteprobe/siteprobe

# pip / pipx
pipx install siteprobe
pip install siteprobe

# Cargo
cargo install siteprobe
```

To run it once without installing:

```sh
uvx siteprobe https://example.com/sitemap.xml
pipx run siteprobe https://example.com/sitemap.xml
```

Prebuilt binaries for macOS, Linux, and Windows are attached to every
[GitHub release](https://github.com/bartTC/siteprobe/releases). Building from
source requires Rust 1.86 or newer:

```sh
git clone https://github.com/bartTC/siteprobe.git
cd siteprobe
cargo build --release
```

## Usage

```sh
siteprobe [SOURCE] [--url <URL>]... [OPTIONS]
```

`SOURCE` is where the URLs come from: a `sitemap.xml`, a sitemap index, or a
plain-text list with one URL per line. It can be an http(s) URL, a local file
path, or `-` to read from stdin. The format is detected automatically. `SOURCE`
is optional when `--url` is used.

### URL sources

```sh
# A sitemap or sitemap index. Nested sitemap indexes are followed recursively.
siteprobe https://example.com/sitemap.xml

# Gzip-compressed sitemaps are detected by their .gz suffix or their content.
siteprobe https://example.com/sitemap.xml.gz

# A plain-text URL list, remote or local. Blank lines and lines starting with #
# are ignored; lines that are not http(s) URLs are skipped with a warning.
siteprobe https://example.com/urls.txt
siteprobe ./urls.txt

# A list or a sitemap from stdin
cat urls.txt | siteprobe -
curl -s https://example.com/sitemap.xml | siteprobe -

# URLs given directly. -u can be repeated and combined with a SOURCE;
# all URLs are merged and deduplicated.
siteprobe -u https://example.com/ -u https://example.com/about
siteprobe https://example.com/sitemap.xml -u https://example.com/new-page
```

### Examples

```sh
# Ten concurrent requests with a five second timeout
siteprobe https://example.com/sitemap.xml --concurrency-limit 10 --request-timeout 5

# List every page slower than one second, and exit with code 3 if there are any
siteprobe https://example.com/sitemap.xml --slow-threshold 1

# Stay under 300 requests per five minutes
siteprobe https://example.com/sitemap.xml --rate-limit 300/5m

# Write a CSV report and keep a copy of every downloaded page
siteprobe https://example.com/sitemap.xml --report-path ./results/report.csv --output-dir ./example.com

# Bypass caches by appending a random timestamp to each URL
siteprobe https://example.com/sitemap.xml --append-timestamp

# Check a few pages without a sitemap
siteprobe -u https://example.com/ -u https://example.com/pricing --slow-threshold 1
```

## Options

```
Usage: siteprobe [OPTIONS] <SOURCE|--url <URL>>

Arguments:
  [SOURCE]  Where to load the URLs from: a sitemap.xml or a plain-text list with
            one URL per line, given as an http(s) URL, a local file path, or '-'
            to read from stdin. Optional if --url is used.

Options:
  -u, --url <URL>
          A URL to probe directly, without loading a sitemap or list. Can be
          specified multiple times and combined with SOURCE.
      --basic-auth <BASIC_AUTH>
          Basic authentication credentials in the format `username:password`
  -H, --header <HEADERS>
          Custom header to include in each request (format: 'Name: Value'). Can
          be specified multiple times.
  -c, --concurrency-limit <CONCURRENCY_LIMIT>
          Maximum number of concurrent requests allowed [default: 4]
  -l, --rate-limit <RATE_LIMIT>
          The rate limit for all requests in the format 'requests/time[unit]',
          where unit can be seconds (`s`), minutes (`m`), or hours (`h`). E.g.
          '-l 300/5m' for 300 requests per 5 minutes, or '-l 100/1h' for 100
          requests per hour.
  -o, --output-dir <OUTPUT_DIR>
          Directory where all downloaded documents will be saved
  -a, --append-timestamp
          Append a random timestamp to each URL to bypass caching mechanisms
  -r, --report-path <REPORT_PATH>
          File path for storing the generated `report.csv`
  -j, --report-path-json <REPORT_PATH_JSON>
          File path for storing the generated `report.json`
      --report-path-html <REPORT_PATH_HTML>
          File path for storing the generated `report.html`
  -t, --request-timeout <REQUEST_TIMEOUT>
          Default timeout (in seconds) for each request [default: 10]
      --user-agent <USER_AGENT>
          Custom User-Agent header to be used in requests [default: "Mozilla/5.0
          (compatible; Siteprobe/1.5.0)"]
      --slow-num <SLOW_NUM>
          Limit the number of slow documents displayed in the report. [default:
          100]
  -s, --slow-threshold <SLOW_THRESHOLD>
          Show slow responses. The value is the threshold (in seconds) for
          considering a document as 'slow'. E.g. '-s 3' for 3 seconds or '-s
          0.05' for 50ms.
  -f, --follow-redirects
          Controls automatic redirects. When enabled, the client will follow
          HTTP redirects (up to 10 by default). Note that for security, Basic
          Authentication credentials are intentionally not forwarded during
          redirects to prevent unintended credential exposure.
      --retries <RETRIES>
          Number of retries for failed requests (network errors or 5xx
          responses) [default: 0]
      --exit-zero
          Always exit with status code 0 after a completed run, even if URLs
          failed or exceeded the slow threshold. Fatal errors, such as an
          unreadable source, still exit non-zero.
      --json
          Output the JSON report to stdout instead of the normal table output.
          Suppresses all other console output for clean piping.
      --config <CONFIG>
          Path to a TOML config file. Defaults to `.siteprobe.toml` in the
          current directory.
  -h, --help
          Print help
  -V, --version
          Print version

Exit Codes:
  0  All URLs returned 2xx (success)
  1  One or more URLs returned 4xx/5xx or failed, or a fatal error occurred
  2  Invalid command line arguments
  3  One or more URLs exceeded the slow threshold (--slow-threshold)

Use --exit-zero to always exit 0 after a completed run.
```

## Authentication and custom headers

```sh
# Basic Authentication
siteprobe https://example.com/sitemap.xml --basic-auth user:password

# Bearer token (via custom header)
siteprobe https://example.com/sitemap.xml -H "Authorization: Bearer <token>"

# Session cookie
siteprobe https://example.com/sitemap.xml -H "Cookie: sessionid=abc123def456"

# Several headers at once
siteprobe https://example.com/sitemap.xml \
  -H "Authorization: Bearer <token>" \
  -H "Cookie: sessionid=abc123" \
  -H "X-Custom-Header: value"
```

If both `--basic-auth` and `-H "Authorization: ..."` are given, the `-H` value
takes precedence. When redirects are followed (`--follow-redirects`), Basic
Authentication credentials are not forwarded to the redirect target.

## Configuration file

Options can be stored in a TOML file so they do not have to be repeated on every
run. Siteprobe reads `.siteprobe.toml` from the current directory if it exists,
or the file given with `--config`. Command-line flags take precedence over the
file, and the file takes precedence over the built-in defaults.

Every key is optional and corresponds to a command-line option. The file below
lists all of them: the active lines are the defaults, the commented lines show
options that have no default.

```toml
# .siteprobe.toml

# Requests
concurrency_limit = 4          # concurrent requests
request_timeout = 10           # seconds per request
retries = 0                    # retries for network errors and 5xx responses
follow_redirects = false       # follow up to 10 redirects
append_timestamp = false       # append a random timestamp to each URL to bypass caches
# rate_limit = "300/5m"        # requests per time span; units: s, m, h
# user_agent = "Mozilla/5.0 (compatible; Siteprobe/1.5.0)"   # the default carries the version

# Authentication
# basic_auth = "user:password"
# headers = ["Authorization: Bearer <token>", "Cookie: sessionid=abc123"]

# Slow responses
slow_num = 100                 # number of slow responses shown in the report
# slow_threshold = 1.0         # seconds; responses above it are reported and exit with code 3

# Reports (none are written unless a path is set)
# report_path = "report.csv"
# report_path_json = "report.json"
# report_path_html = "report.html"

# Exit code
exit_zero = false              # exit 0 even if URLs failed or were slow
```

## Reports

Every run prints statistics to the terminal: success, error, and redirect
rates; response time percentiles; throughput and response sizes; and the
responses above `--slow-threshold`, limited to `--slow-num` entries.

- `--report-path <FILE>` writes a CSV file with one row per URL: URL, response
  time in milliseconds, response size, and status code.
- `--report-path-json <FILE>` writes a JSON file with the run configuration
  (`sitemapUrl`, `concurrencyLimit`, `elapsedTime`, `bypassCaching`), the
  statistics, and every response with its `url`, `responseTime`,
  `responseSize`, and `statusCode`.
- `--report-path-html <FILE>` writes a self-contained HTML file with summary
  statistics, a response time histogram, a status code chart, and a sortable
  table of all responses.
- `--json` prints the JSON report to stdout and suppresses all other output:

  ```sh
  siteprobe https://example.com/sitemap.xml --json | jq '.responses[] | select(.statusCode != 200)'
  ```

## Exit codes

| Code | Meaning                                                                                                      |
|------|--------------------------------------------------------------------------------------------------------------|
| `0`  | All URLs returned 2xx.                                                                                       |
| `1`  | One or more URLs returned 4xx/5xx or failed, or a fatal error occurred (e.g. the source could not be loaded). |
| `2`  | Invalid command line arguments.                                                                              |
| `3`  | One or more URLs exceeded the `--slow-threshold`.                                                            |

A run with both failing and slow URLs exits with `1`.

Pass `--exit-zero` (or set `exit_zero = true` in `.siteprobe.toml`) to always
exit with `0` after a completed run, for example when only the report matters
and failing URLs should not fail the job. Fatal errors and invalid arguments
still exit non-zero.

```sh
siteprobe https://example.com/sitemap.xml --report-path-html report.html --exit-zero
```

## Development

Requires Rust 1.86 or newer and [just](https://github.com/casey/just). The
`Justfile` lists all recipes (`just --list`); the most useful ones:

```sh
just check        # cargo fmt --check, cargo clippy -D warnings, cargo test
just test         # run the tests; add --cov for an HTML coverage report (needs cargo-tarpaulin)
just serve-site   # build the website into _site/ and serve it at http://localhost:8000/
```

The website consists of the landing page `docs/index.html`, the shared
stylesheet `docs/site.css`, and this README and the [CHANGELOG](CHANGELOG.md)
rendered with [microdocs](https://github.com/bartTC/microdocs) through the
template `docs/template.html`. It is deployed to GitHub Pages on every push to
`main`.

To release a new version, set it in `Cargo.toml`, describe the changes under
`[Unreleased]` in `CHANGELOG.md`, and run `just release`. This runs the tests,
turns the unreleased section into a dated release entry, commits, tags
`vX.Y.Z`, and pushes. The release workflow on GitHub then builds the binaries
and publishes to GitHub Releases, Homebrew, and PyPI.

## License

MIT, see [LICENSE](LICENSE).
