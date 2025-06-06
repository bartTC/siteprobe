use crate::metrics::{CLEAN_FORMAT, Entry, Metrics};
use crate::options::Cli;
use crate::utils;
use console::style;
use csv::Writer;
use prettytable::{Cell, Row, Table};
use reqwest::StatusCode;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io::Write;
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
    pub rate_limit: Option<u32>,
    pub total_time: Duration,
    pub responses: VecDeque<Response>,
}

#[derive(Debug)]
pub struct Statistics {
    pub response_time: Metrics,
    pub status_code: Metrics,
    pub performance: Metrics,
}

impl Report {
    pub fn show_text_report(&self, options: &Cli) {
        let stats = self.generate_statistics(options.slow_threshold);
        let base_metrics = Metrics(vec![
            Entry {
                label: "Concurrency Limit",
                value: self.concurrency_limit.to_string(),
                json_label: "concurrencyLimit",
                json_value: json!(self.concurrency_limit),
            },
            Entry {
                label: "Rate Limit",
                value: if self.rate_limit.is_some() {
                    format!("{}/min", self.rate_limit.unwrap().to_string())
                } else {
                    "No".to_string()
                },
                json_label: "rateLimit",
                json_value: json!(self.rate_limit),
            },
            Entry {
                label: "Elapsed Time",
                value: format!("{:.2?}", self.total_time),
                json_label: "elapsedTimeMs",
                json_value: json!(self.total_time.as_millis()),
            },
            Entry {
                label: "Bypass Caching",
                value: if options.append_timestamp {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                },
                json_label: "bypassCaching",
                json_value: json!(options.append_timestamp),
            },
        ]);

        println!(
            "\n\n{} {}\n",
            style("Statistics for").bold(),
            style(&self.sitemap_url).bold().underlined()
        );

        let mut table = Table::new();
        table.set_format(*CLEAN_FORMAT);
        table.add_row(Row::new(vec![
            Cell::new(base_metrics.build_table().as_str()),
            Cell::new(stats.status_code.build_table().as_str()),
        ]));
        println!("{}", table);

        println!(
            "{}\n",
            style("Response Time and Performance Statistics:").bold()
        );

        let mut table = Table::new();
        table.set_format(*CLEAN_FORMAT);
        table.add_row(Row::new(vec![
            Cell::new(stats.response_time.build_table().as_str()),
            Cell::new(stats.performance.build_table().as_str()),
        ]));
        println!("{}", table);

        // Error Response List
        let error_responses = self.error_responses();
        if !error_responses.is_empty() {
            println!("{}\n", style("Error Responses:").bold());
            for r in error_responses {
                println!(
                    "{} {} {}",
                    if r.status_code.is_server_error() {
                        style(format!("{}:", r.status_code)).bold().white().on_red()
                    } else {
                        style(format!("{}:", r.status_code)).bold().dim()
                    },
                    r.url,
                    style(format!("{}ms", r.response_time.as_millis())).dim()
                );
            }
            println!(); // Blank line before slow responses
        }

        // Slow Response List
        if let Some(threshold) = options.slow_threshold {
            let slow_responses = self.slowest_responses(threshold, options.slow_num);
            if !slow_responses.is_empty() {
                println!(
                    "{} {}\n",
                    style("Slow Responses:").bold(),
                    style(format!(">={}s", threshold)).dim().italic()
                );
                for r in slow_responses {
                    println!(
                        "{} {} {}",
                        style(format!("{}:", r.status_code)).bold().dim(),
                        r.url,
                        style(format!("{}ms", r.response_time.as_millis())).dim()
                    );
                }
            }
        }
    }

    pub fn write_json_report(
        &self,
        options: &Cli,
        report_path: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        // If the report path parent is a director, create it if it doesn't exist yet
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let statistics = self.generate_statistics(options.slow_threshold);

        let json_data = json!(
            {
               "config": {
                    "sitemapUrl": self.sitemap_url,
                    "concurrencyLimit": self.concurrency_limit,
                    "elapsedTime": self.total_time.as_millis(),
                    "bypassCaching": options.append_timestamp,
                },
                "statistics": {
                    "performance": statistics.performance,
                    "responseTime": statistics.response_time,
                    "statusCode": statistics.status_code,
                },
                "responses" : self.responses.iter().map(|r| {
                    json!({
                        "url": r.url,
                        "responseTime": r.response_time.as_millis(),
                        "responseSize": r.response_size,
                        "statusCode": r.status_code.as_u16(),
                    })
                }).collect::<Vec<serde_json::Value>>()
            }
        );

        // Write the JSON to a file
        let mut file = File::create(report_path)?;
        file.write_all(serde_json::to_string_pretty(&json_data)?.as_bytes())?;

        println!(
            "\n📄 The JSON report was written to {}",
            style(report_path.display()).underlined().cyan()
        );

        Ok(())
    }

    /// Write a CSV report
    pub fn write_csv_report(&self, report_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        // If the report path parent is a director, create it if it doesn't exist yet
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

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

    fn generate_statistics(&self, slow_threshold: Option<f64>) -> Statistics {
        let report = &self;
        let total_requests = report.responses.len();
        let total_time_secs = report.total_time.as_secs_f64();

        let response_times: Vec<Duration> =
            report.responses.iter().map(|r| r.response_time).collect();
        let response_sizes: Vec<usize> = report.responses.iter().map(|r| r.response_size).collect();

        let avg_response_time =
            response_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / total_requests as f64;
        let median_response_time = response_times.get(response_times.len() / 2).copied();
        let min_response_time = response_times.iter().copied().min();
        let max_response_time = response_times.iter().copied().max();
        let p90_response_time = response_times
            .get((response_times.len() as f64 * 0.90) as usize)
            .copied();
        let p95_response_time = response_times
            .get((response_times.len() as f64 * 0.95) as usize)
            .copied();
        let p99_response_time = response_times
            .get((response_times.len() as f64 * 0.99) as usize)
            .copied();

        let variance = response_times
            .iter()
            .map(|t| (t.as_secs_f64() - avg_response_time).powi(2))
            .sum::<f64>()
            / total_requests as f64;
        let std_dev = variance.sqrt();

        let mut status_counts: HashMap<StatusCode, usize> = HashMap::new();
        let mut success_count = 0;
        let mut error_count = 0;
        let mut redirect_count = 0;
        let mut slow_count = 0;

        for response in &report.responses {
            *status_counts.entry(response.status_code).or_insert(0) += 1;
            if response.status_code.is_success() {
                success_count += 1;
            } else if response.status_code.is_client_error()
                || response.status_code.is_server_error()
            {
                error_count += 1;
            } else if response.status_code.is_redirection() {
                redirect_count += 1;
            }

            if let Some(threshold) = slow_threshold {
                if response.response_time.as_secs_f64() > threshold {
                    slow_count += 1;
                }
            }
        }

        let success_rate = (success_count as f64 / total_requests as f64) * 100.0;
        let error_rate = (error_count as f64 / total_requests as f64) * 100.0;
        let redirect_rate = (redirect_count as f64 / total_requests as f64) * 100.0;
        let slow_request_percentage = (slow_count as f64 / total_requests as f64) * 100.0;

        let avg_response_size = response_sizes.iter().sum::<usize>() / total_requests;
        let min_response_size = response_sizes.iter().copied().min();
        let max_response_size = response_sizes.iter().copied().max();

        Statistics {
            response_time: Metrics(vec![
                Entry {
                    label: "⏱️ Average Response Time",
                    value: utils::ms(Duration::from_secs_f64(avg_response_time)),
                    json_label: "avgMs",
                    json_value: json!(Duration::from_secs_f64(avg_response_time).as_millis()),
                },
                Entry {
                    label: "🔷 Median Response Time",
                    value: utils::ms(median_response_time.unwrap_or_default()),
                    json_label: "medianMs",
                    json_value: json!(median_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "🐇 Min Response Time",
                    value: utils::ms(min_response_time.unwrap_or_default()),
                    json_label: "minMs",
                    json_value: json!(min_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "🐌 Max Response Time",
                    value: utils::ms(max_response_time.unwrap_or_default()),
                    json_label: "maxMs",
                    json_value: json!(max_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "⚖️ P90 Response Time",
                    value: utils::ms(p90_response_time.unwrap_or_default()),
                    json_label: "p90Ms",
                    json_value: json!(p90_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "🎯 P95 Response Time",
                    value: utils::ms(p95_response_time.unwrap_or_default()),
                    json_label: "p95Ms",
                    json_value: json!(p95_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "🚀 P99 Response Time",
                    value: utils::ms(p99_response_time.unwrap_or_default()),
                    json_label: "p99Ms",
                    json_value: json!(p99_response_time.unwrap_or_default().as_millis()),
                },
                Entry {
                    label: "📉 Standard Deviation",
                    value: utils::ms(Duration::from_secs_f64(std_dev)),
                    json_label: "stdDevMs",
                    json_value: json!(Duration::from_secs_f64(std_dev).as_millis()),
                },
            ]),
            status_code: Metrics(vec![
                Entry {
                    label: "✅ Success Rate",
                    value: utils::percent(success_rate),
                    json_label: "successRatePercentage",
                    json_value: json!(success_rate),
                },
                Entry {
                    label: "🚨 Error Rate",
                    value: utils::percent(error_rate),
                    json_label: "errorRatePercentage",
                    json_value: json!(error_rate),
                },
                Entry {
                    label: "🔄 Redirect Rate",
                    value: utils::percent(redirect_rate),
                    json_label: "redirectRatePercentage",
                    json_value: json!(redirect_rate),
                },
            ]),
            performance: Metrics(vec![
                Entry {
                    label: "⚡️ Total Requests Processed",
                    value: total_requests.to_string(),
                    json_label: "totalRequests",
                    json_value: json!(total_requests),
                },
                Entry {
                    label: "⏳ Requests Per Second (RPS)",
                    value: if total_time_secs > 0.0 {
                        format!("{:.02} / sec", total_requests as f64 / total_time_secs)
                    } else {
                        "0 / sec".to_string()
                    },
                    json_label: "requestsPerSecond",
                    json_value: json!(total_requests as f64 / total_time_secs),
                },
                Entry {
                    label: "📉 Slow Request Percentage",
                    value: if slow_threshold.is_some() {
                        utils::percent(slow_request_percentage)
                    } else {
                        "Not Set".to_string()
                    },
                    json_label: "slowRequestPercentage",
                    json_value: json!(slow_request_percentage),
                },
                Entry {
                    label: "📦 Average Response Size",
                    value: utils::kb(avg_response_size),
                    json_label: "avgResponseSizeBytes",
                    json_value: json!(avg_response_size),
                },
                Entry {
                    label: "🔹 Min Response Size",
                    value: utils::kb(min_response_size.unwrap_or_default()),
                    json_label: "minResponseSizeBytes",
                    json_value: json!(min_response_size.unwrap_or_default()),
                },
                Entry {
                    label: "🔺 Max Response Size",
                    value: utils::kb(max_response_size.unwrap_or_default()),
                    json_label: "maxResponseSizeBytes",
                    json_value: json!(max_response_size.unwrap_or_default()),
                },
            ]),
        }
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
    fn slowest_responses(&self, threshold: f64, limit: u32) -> Vec<Response> {
        let mut responses: Vec<_> = self
            .responses
            .iter()
            .filter(|r| r.response_time.as_secs_f64() >= threshold)
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
