# Siteprobe

Siteprobe is a Rust-based CLI tool that fetches all URLs from a given `sitemap.xml`
url, checks their existence, and generates a performance report. It supports various
features such as authentication, concurrency control, caching bypass, and more.

## Features

- Fetch and parse `sitemap.xml` to extract URLs
- Check the existence and response times of each URL
- Generate a detailed performance report (`report.csv`)
- Support for Basic Authentication
- Adjustable concurrency limits for request handling
- Configurable request timeout settings
- Custom User-Agent header support
- Option to append random timestamps to URLs to bypass caching mechanisms
- Redirect handling with security precautions
- Filtering and reporting slow URLs based on a threshold
- Save downloaded documents for further inspection

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

```bash 
$ siteprobe --help
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
  -t, --request-timeout <REQUEST_TIMEOUT>
          Default timeout (in seconds) for each request [default: 10]
      --user-agent <USER_AGENT>
          Custom User-Agent header to be used in requests 
          [default: "Mozilla/5.0 (compatible; fetch-sitemap/0.1.0)"]
      --slow-num <SLOW_NUM>
          Limit the number of slow documents displayed in the report [default: No limit]
  -s, --slow-threshold <SLOW_THRESHOLD>
          Threshold (in seconds) for considering a document as 'slow'. [default: 3]
  -f, --follow-redirects
          Controls automatic redirects. When enabled, the client will follow HTTP 
          redirects (up to 10 by default). Note that for security, Basic Authentication 
          credentials are intentionally not forwarded during redirects to prevent 
          unintended credential exposure.
  -h, --help
          Print help
```

### Example Usage

#### Fetch and analyze a sitemap with default settings

```sh
siteprobe https://example.com/sitemap.xml
```

#### Set concurrency limit to 50 and timeout to 5 seconds

```sh
siteprobe https://example.com/sitemap.xml --concurrency-limit 50 --request-timeout 5
```

#### Save the report to a specific file

```sh
siteprobe https://example.com/sitemap.xml --report-path ./results/report.csv
```

#### Append timestamps to bypass cache and follow redirects

```sh
siteprobe https://example.com/sitemap.xml --append-timestamp --follow-redirects
```

