# Siteprobe

Siteprobe is a Rust-based CLI tool that fetches all URLs from a given `sitemap.xml`
or a plain list of URLs, checks their existence, and generates a performance report.
It supports various features such as authentication, concurrency control, caching
bypass, and more.

![Screenshot of Siteprobe statistics](https://github.com/bartTC/siteprobe/blob/main/docs/screenshot.png?raw=true)

## Features

- Fetch and parse sitemap.xml to extract URLs, including nested Sitemap Index files
  recursively.
- Alternatively read a plain-text list of URLs, from a URL, a local file, or stdin,
  or pass individual URLs directly on the command line.
- Check the existence and response times of each URL.
- Generate a detailed performance CSV report.
- Support for Basic Authentication.
- Adjustable concurrency limits for request handling.
- Configurable request timeout settings.
- Support for configuring rate limits, such as 300 requests per 5-minute interval.
- Redirect handling with security precautions.
- Filtering and reporting slow URLs based on a threshold.
- Custom User-Agent header support.
- Option to append random timestamps to URLs to bypass caching mechanisms.
- Save downloaded documents for further inspection or use as a static site mirror.

## Installation

### Run without installing

```sh
uvx siteprobe https://example.com/sitemap.xml
# or
pipx run siteprobe https://example.com/sitemap.xml
```

### Install via package manager

```sh
# Homebrew (macOS/Linux)
brew install bartTC/siteprobe/siteprobe

# pip / pipx
pip install siteprobe
pipx install siteprobe

# Cargo
cargo install siteprobe
```

### Build from source

Building from source requires Rust 1.86 or newer.

```sh
git clone https://github.com/bartTC/siteprobe.git
cd siteprobe
cargo build --release
```

## Usage

```sh
siteprobe [SOURCE] [--url <URL>]... [OPTIONS]
```

### Arguments

- `[SOURCE]` - Where to load the URLs from. Either a `sitemap.xml` (or sitemap
  index) or a plain-text list with one URL per line, given as an http(s) URL, a
  local file path, or `-` to read from stdin. The format is detected automatically.
  Optional if `--url` is used.
- `-u, --url <URL>` - A URL to probe directly. Can be repeated and combined with
  `SOURCE`; all URLs are merged and deduplicated.

### URL Sources

Besides a `sitemap.xml`, siteprobe accepts a plain-text list of URLs. Blank lines
and lines starting with `#` are ignored; lines that are not http(s) URLs are
skipped with a warning.

```sh
# A sitemap or sitemap index
siteprobe https://example.com/sitemap.xml

# A plain-text URL list, remote or local
siteprobe https://example.com/urls.txt
siteprobe ./urls.txt

# Read the list (or a sitemap) from stdin
cat urls.txt | siteprobe -
curl -s https://example.com/sitemap.xml | siteprobe -

# Probe a handful of URLs directly
siteprobe -u https://example.com/ -u https://example.com/about

# Combine a source with extra URLs
siteprobe https://example.com/sitemap.xml -u https://example.com/new-page
```

### Options

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
  1  One or more URLs returned 4xx/5xx or failed
  2  One or more URLs exceeded the slow threshold (--slow-threshold)
```

### Authentication & Custom Headers

Siteprobe supports several ways to authenticate requests:

```sh
# Basic Authentication
siteprobe https://example.com/sitemap.xml --basic-auth user:password

# Bearer token (via custom header)
siteprobe https://example.com/sitemap.xml -H "Authorization: Bearer <token>"

# Send a session cookie
siteprobe https://example.com/sitemap.xml -H "Cookie: sessionid=abc123def456"
```

You can combine multiple `-H` flags to send several custom headers at once:

```sh
siteprobe https://example.com/sitemap.xml \
  -H "Authorization: Bearer <token>" \
  -H "Cookie: sessionid=abc123" \
  -H "X-Custom-Header: value"
```

If both `--basic-auth` and `-H "Authorization: ..."` are provided, the `-H` value
takes precedence.

### Example Usage

```sh
# Fetch and analyze a sitemap with default settings
siteprobe https://example.com/sitemap.xml

# Save the report to a specific file
siteprobe https://example.com/sitemap.xml --report-path ./results/report.csv --output-dir ./example.com

# Set concurrency limit to 10 and timeout to 5 seconds
siteprobe https://example.com/sitemap.xml --concurrency-limit 10 --request-timeout 5

# Quickly check a few pages without a sitemap
siteprobe -u https://example.com/ -u https://example.com/pricing --slow-threshold 1

# Check the URLs listed in a local text file
siteprobe ./urls.txt --report-path ./results/report.csv
```
