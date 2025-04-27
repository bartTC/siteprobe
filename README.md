# Siteprobe

Siteprobe is a Rust-based CLI tool that fetches all URLs from a given `sitemap.xml`
url, checks their existence, and generates a performance report. It supports various
features such as authentication, concurrency control, caching bypass, and more.

![Screenshot of Siteprobe statistics](https://github.com/bartTC/siteprobe/blob/main/docs/screenshot.png?raw=true)

## Features

- Fetch and parse sitemap.xml to extract URLs, including nested Sitemap Index files
  recursively.
- Check the existence and response times of each URL
- Generate a detailed performance report (e.g. `report.csv`)
- Support for Basic Authentication
- Adjustable concurrency limits for request handling
- Configurable request timeout settings
- Custom User-Agent header support
- Option to append random timestamps to URLs to bypass caching mechanisms
- Redirect handling with security precautions
- Filtering and reporting slow URLs based on a threshold
- Save downloaded documents for further inspection or use as a static site mirror.

## Installation

You can install Siteprobe using Cargo:

```sh
cargo install siteprobe
```

Alternatively, build from source:

```sh
git clone https://github.com/bartTC/siteprobe.git
cd siteprobe
cargo build --release
```

## Usage

```sh
siteprobe <sitemap_url> [OPTIONS]
```

### Arguments

- `<sitemap_url>` - The URL of the sitemap to be fetched and processed.

### Options

```
Usage: siteprobe [OPTIONS] <SITEMAP_URL>

Arguments:
  <SITEMAP_URL>  The URL of the sitemap to be fetched and processed.

Options:
      --basic-auth <BASIC_AUTH>
          Basic authentication credentials in the format `username:password`
  -c, --concurrency-limit <CONCURRENCY_LIMIT>
          Maximum number of concurrent requests allowed [default: 4]
  -o, --output-dir <OUTPUT_DIR>
          Directory where all downloaded documents will be saved
  -a, --append-timestamp
          Append a random timestamp to each URL to bypass caching mechanisms
  -r, --report-path <REPORT_PATH>
          File path for storing the generated `report.csv`
  -j, --report-path-json <REPORT_PATH_JSON>
          File path for storing the generated `report.json`
  -t, --request-timeout <REQUEST_TIMEOUT>
          Default timeout (in seconds) for each request [default: 10]
      --user-agent <USER_AGENT>
          Custom User-Agent header to be used in requests [default: "Mozilla/5.0
          (compatible; Siteprobe/0.3.0)"]
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
  -h, --help
          Print help
```

### Example Usage

```sh
# Fetch and analyze a sitemap with default settings
siteprobe https://example.com/sitemap.xml

# Save the report to a specific file
siteprobe https://example.com/sitemap.xml --report-path ./results/report.csv --output-dir ./example.com

# Set concurrency limit to 10 and timeout to 5 seconds
siteprobe https://example.com/sitemap.xml --concurrency-limit 10 --request-timeout 5
```
