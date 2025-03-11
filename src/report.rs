use crate::metrics::{Entry, Metrics};
use crate::options::Cli;
use console::style;
use csv::Writer;
use reqwest::StatusCode;
use std::collections::VecDeque;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Response {
    pub url: String,
    pub response_time: Duration,
    pub response_size: usize,
    pub status_code: StatusCode,
}

#[derive(Debug)]
pub struct Report {
    pub sitemap_url: String,
    pub concurrency_limit: u8,
    pub total_time: Duration,
    pub responses: VecDeque<Response>,
}

impl Report {
    pub fn show_text_report(&self, options: &Cli) {
        println!("\n\n{}", self.statistics_metrics().build_table());
        println!("{}", self.response_time_metrics().unwrap().build_table());

        for r in self.error_responses() {
            println!(
                "{}: {} ({}ms)",
                style(r.status_code).bold(),
                r.url,
                r.response_time.as_millis()
            );
        }

        for r in self.slowest_responses(options.slow_threshold, 10i32) {
            println!(
                "{}: {} ({}ms)",
                style(r.status_code).bold(),
                r.url,
                r.response_time.as_millis()
            );
        }
        let tip =
            style("💡 Tip: You can adjust the threshold for slow responses using the `-s` flag.")
                .dim()
                .italic();
        println!("\n{}", tip);
    }

    /// Write a CSV report
    pub fn write_csv_report(&self, report_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let mut writer = Writer::from_path(report_path)?;
        writer.write_record(vec![
            "URL",
            "Response Time (ms)",
            "Response Size",
            "Status Code",
        ])?;
        for r in &self.responses {
            writer.write_record(vec![
                &r.url,
                &r.response_time.as_millis().to_string(),
                &r.response_size.to_string(),
                &r.status_code.to_string(),
            ])?;
        }
        println!(
            "\n📊 The CSV report was written to {}",
            style(report_path.display()).underlined().cyan()
        );

        Ok(())
    }

    // === Statistics ==============================================================================

    /// Returns a `Metrics` object containing statistical information about a `Report`.
    ///
    /// The metrics include the following fields:
    /// - **Sitemap URL**: The URL of the sitemap being processed.
    /// - **Concurrency Limit**: The maximum number of HTTP requests allowed to execute concurrently.
    /// - **Elapsed Time**: The total duration of the processing, in seconds, formatted to two decimal places.
    /// - **Total Requests**: The total number of HTTP responses recorded.
    ///
    /// This function aggregates these details into `Entry` objects, which are
    /// stored in a `Metrics` collection, which can be used for reporting or debugging.
    ///
    /// # Returns
    /// A `Metrics` object containing entries for the statistical information of the `Report`.
    fn statistics_metrics(&self) -> Metrics {
        Metrics(vec![
            Entry {
                label: "Sitemap URL",
                value: self.sitemap_url.to_string(),
            },
            Entry {
                label: "Concurrency Limit",
                value: self.concurrency_limit.to_string(),
            },
            Entry {
                label: "Elapsed Time",
                value: format!("{:.2?}", self.total_time),
            },
            Entry {
                label: "Total Requests",
                value: self.responses.len().to_string(),
            },
        ])
    }

    /// Calculates and retrieves response time metrics for a collection of responses.
    ///
    /// This function processes the response times from the stored responses,
    /// and returns a set of metrics encapsulated in a `Metrics` struct. The metrics include:
    ///
    /// - **Average Response Time**: The mean response time.
    /// - **P99 Response Time**: The response time of the 99th percentile (fastest 99% of requests).
    /// - **P95 Response Time**: The response time of the 95th percentile (fastest 95% of requests).
    /// - **Minimum Response Time**: The fastest recorded response time.
    /// - **Maximum Response Time**: The slowest recorded response time.
    ///
    /// The metrics are sorted and calculated using nanosecond precision, ensuring accuracy
    /// while clamping values to fit within valid ranges for `u64`.
    ///
    /// # Returns
    ///
    /// - `Some(Metrics)` containing response time metrics as a vector of `LabeledValue`s
    ///   if there are any recorded responses.
    /// - `None` if the response collection is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(metrics) = report.get_response_time_metrics() {
    ///     for metric in metrics.0 {
    ///         println!("{}: {:?}", metric.label, metric.value);
    ///     }
    /// }
    /// // Output:
    /// // ⏱️ Average Response Time: 300ms
    /// // 🚀 P99 Response Time: 400ms
    /// // ⭐️ P95 Response Time: 400ms
    /// // 🐅 Min Response Time: 200ms
    /// // 🐌 Max Response Time: 400ms
    /// ```
    ///
    /// # Errors
    ///
    /// This function does not return errors explicitly. It ensures safe calculations
    /// and clamps values to avoid integer overflows.
    fn response_time_metrics(&self) -> Option<Metrics> {
        if self.responses.is_empty() {
            return None;
        }

        let mut response_times: Vec<u128> = self
            .responses
            .iter()
            .map(|r| r.response_time.as_nanos())
            .collect();

        response_times.sort_unstable(); // Sort in ascending order (fastest to slowest)

        let fastest: u64 = response_times
            .iter()
            .min()
            .map(|&time| time.clamp(0, u64::MAX as u128) as u64) // Ensure the value is in range
            .unwrap_or_default(); // Default to 0 if iterator is empty

        let slowest: u64 = response_times
            .iter()
            .max()
            .map(|&time| time.clamp(0, u64::MAX as u128) as u64) // Ensure the value is in range
            .unwrap_or_default(); // Default to 0 if iterator is empty

        let total_nanos: u128 = response_times.iter().sum();
        let avg_nanos = (total_nanos as f64 / response_times.len() as f64).round() as u64;

        // Calculate the indices for P99 and P95
        let p99_index =
            ((response_times.len() as f64 * 0.99).floor() as usize).min(response_times.len() - 1);
        let p95_index =
            ((response_times.len() as f64 * 0.95).floor() as usize).min(response_times.len() - 1);

        let p99_fastest_nanos = response_times[p99_index] as u64;
        let p95_fastest_nanos = response_times[p95_index] as u64;

        fn format_duration(d: Duration) -> String {
            let secs = d.as_secs();
            let millis = d.subsec_millis();
            format!("{}.{:03}s", secs, millis)
        }

        Some(Metrics(vec![
            Entry {
                label: "⏱️ Average Response Time",
                value: format_duration(Duration::from_nanos(avg_nanos)),
            },
            Entry {
                label: "🚀 P99 Response Time",
                value: format_duration(Duration::from_nanos(p99_fastest_nanos)),
            },
            Entry {
                label: "⭐️ P95 Response Time",
                value: format_duration(Duration::from_nanos(p95_fastest_nanos)),
            },
            Entry {
                label: "🐅 Min Response Time",
                value: format_duration(Duration::from_nanos(fastest)),
            },
            Entry {
                label: "🐌 Max Response Time",
                value: format_duration(Duration::from_nanos(slowest)),
            },
        ]))
    }

    /// Filters and retrieves the slowest HTTP responses from the report.
    ///
    /// This function identifies HTTP responses with a response time exceeding the specified
    /// threshold and sorts them in descending order of their response times. The output is
    /// limited to the specified number of responses.
    ///
    /// # Arguments
    ///
    /// * `threshold` - A `f64` value (measured in seconds) that represents the minimum
    ///   response time used to filter responses. Only responses with a `response_time`
    ///   greater than this value will be included.
    /// * `limit` - An `i32` value representing the maximum number of slow responses to include
    ///   in the resulting vector.
    ///
    /// # Returns
    ///
    /// A `Vec<Response>` containing at most `limit` responses sorted by `response_time`
    /// in descending order. Each response in the vector has a `response_time` greater
    /// than the given threshold.
    fn slowest_responses(&self, threshold: f64, limit: i32) -> Vec<Response> {
        let mut responses: Vec<_> = self
            .responses
            .iter()
            .filter(|r| r.response_time.as_secs_f64() > threshold)
            .cloned()
            .collect();
        responses.sort_unstable_by(|a, b| b.response_time.cmp(&a.response_time));
        responses.into_iter().take(limit as usize).collect()
    }

    /// Filters and returns a sorted list of error responses from the report.
    ///
    /// # Description
    /// This function processes the `responses` field of the `Report` struct to extract
    /// all responses whose HTTP status codes indicate either client errors (4xx)
    /// or server errors (5xx). The resulting list is then sorted primarily by
    /// status code in descending order, and secondarily by URL in ascending order.
    ///
    /// # Returns
    /// A `Vec<Response>` containing the filtered and sorted error responses.
    ///
    /// # Sorting
    /// 1. **Primary**: Status code (descending).
    /// 2. **Secondary**: URL (ascending).
    ///
    /// # See Also
    /// `Response` - Contains details about individual HTTP requests, such as the
    /// URL, status code, response time, etc.
    fn error_responses(&self) -> Vec<Response> {
        let mut responses: Vec<_> = self
            .responses
            .iter()
            .filter(|r| r.status_code.is_client_error() || r.status_code.is_server_error())
            .cloned()
            .collect();

        // Order by status code desc, then url asc.
        responses.sort_unstable_by(|a, b| {
            b.status_code
                .cmp(&a.status_code)
                .then_with(|| a.url.cmp(&b.url))
        });
        responses
    }
}
