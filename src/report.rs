use crate::metrics::{Entry, Metrics, CLEAN_FORMAT};
use crate::options::Cli;
use crate::utils;
use console::style;
use csv::Writer;
use prettytable::{Cell, Row, Table};
use reqwest::StatusCode;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

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
                    format!("{}/min", self.rate_limit.unwrap())
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

    fn build_json_data(&self, options: &Cli) -> serde_json::Value {
        let statistics = self.generate_statistics(options.slow_threshold);

        json!(
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
        )
    }

    /// Returns the JSON report as a pretty-printed string.
    pub fn to_json_string(&self, options: &Cli) -> Result<String, Box<dyn Error>> {
        let json_data = self.build_json_data(options);
        Ok(serde_json::to_string_pretty(&json_data)?)
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

        let json_data = self.build_json_data(options);

        // Write the JSON to a file
        let mut file = File::create(report_path)?;
        file.write_all(serde_json::to_string_pretty(&json_data)?.as_bytes())?;

        if !options.json {
            println!(
                "\n📄 The JSON report was written to {}",
                style(report_path.display()).underlined().cyan()
            );
        }

        Ok(())
    }

    /// Write a CSV report
    pub fn write_csv_report(&self, report_path: &PathBuf, quiet: bool) -> Result<(), Box<dyn Error>> {
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
        if !quiet {
            println!(
                "\n📊 The CSV report was written to {}",
                style(report_path.display()).underlined().cyan()
            );
        }

        Ok(())
    }

    /// Write a self-contained HTML report
    pub fn write_html_report(
        &self,
        options: &Cli,
        report_path: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let stats = self.generate_statistics(options.slow_threshold);
        let total_requests = self.responses.len();
        let total_time_secs = self.total_time.as_secs_f64();

        // Build status code counts for the bar chart
        let mut status_counts: HashMap<u16, usize> = HashMap::new();
        for r in &self.responses {
            *status_counts.entry(r.status_code.as_u16()).or_insert(0) += 1;
        }
        let mut status_entries: Vec<(u16, usize)> = status_counts.into_iter().collect();
        status_entries.sort_by_key(|&(code, _)| code);

        // Build histogram buckets for response time distribution
        let times_ms: Vec<f64> = self
            .responses
            .iter()
            .map(|r| r.response_time.as_secs_f64() * 1000.0)
            .collect();
        let (histogram_svg, histogram_buckets_exist) = if !times_ms.is_empty() {
            let min_t = times_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_t = times_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let bucket_count = 20usize;
            let range = if (max_t - min_t).abs() < 0.001 {
                1.0
            } else {
                max_t - min_t
            };
            let bucket_width = range / bucket_count as f64;
            let mut buckets = vec![0usize; bucket_count];
            for &t in &times_ms {
                let idx = ((t - min_t) / bucket_width).floor() as usize;
                let idx = idx.min(bucket_count - 1);
                buckets[idx] += 1;
            }
            let max_count = *buckets.iter().max().unwrap_or(&1);
            let chart_w = 600.0f64;
            let chart_h = 200.0f64;
            let bar_w = chart_w / bucket_count as f64;

            let mut svg = format!(
                r#"<svg viewBox="0 0 {vw} {vh}" xmlns="http://www.w3.org/2000/svg" style="width:100%;max-width:700px">"#,
                vw = chart_w + 80.0,
                vh = chart_h + 50.0
            );
            // Y axis label
            svg.push_str(&format!(
                r##"<text x="10" y="{}" font-size="11" fill="#64748b" text-anchor="middle" transform="rotate(-90,10,{})">Count</text>"##,
                chart_h / 2.0 + 10.0,
                chart_h / 2.0 + 10.0
            ));
            for (i, &count) in buckets.iter().enumerate() {
                let bar_h = if max_count > 0 {
                    (count as f64 / max_count as f64) * chart_h
                } else {
                    0.0
                };
                let x = 40.0 + i as f64 * bar_w;
                let y = chart_h - bar_h + 10.0;
                svg.push_str(&format!(
                    r##"<rect x="{x:.1}" y="{y:.1}" width="{bw:.1}" height="{bh:.1}" fill="#6366f1" opacity="0.85" rx="1"><title>{lo:.0}-{hi:.0}ms: {count}</title></rect>"##,
                    x = x,
                    y = y,
                    bw = bar_w - 1.0,
                    bh = bar_h,
                    lo = min_t + i as f64 * bucket_width,
                    hi = min_t + (i + 1) as f64 * bucket_width,
                    count = count
                ));
                // X-axis labels (every 4th bucket)
                if i % 4 == 0 || i == bucket_count - 1 {
                    svg.push_str(&format!(
                        r##"<text x="{x:.1}" y="{y}" font-size="10" fill="#64748b" text-anchor="middle">{label:.0}</text>"##,
                        x = x + bar_w / 2.0,
                        y = chart_h + 25.0,
                        label = min_t + i as f64 * bucket_width
                    ));
                }
            }
            // X axis title
            svg.push_str(&format!(
                r##"<text x="{}" y="{}" font-size="11" fill="#64748b" text-anchor="middle">Response Time (ms)</text>"##,
                40.0 + chart_w / 2.0,
                chart_h + 45.0
            ));
            svg.push_str("</svg>");
            (svg, true)
        } else {
            (String::from("<p>No data available.</p>"), false)
        };
        let _ = histogram_buckets_exist;

        // Status code bar chart SVG
        let status_svg = if !status_entries.is_empty() {
            let max_count = status_entries.iter().map(|&(_, c)| c).max().unwrap_or(1);
            let chart_w = 400.0f64;
            let chart_h = 200.0f64;
            let bar_w = chart_w / status_entries.len() as f64;
            let mut svg = format!(
                r#"<svg viewBox="0 0 {vw} {vh}" xmlns="http://www.w3.org/2000/svg" style="width:100%;max-width:500px">"#,
                vw = chart_w + 60.0,
                vh = chart_h + 60.0
            );
            for (i, &(code, count)) in status_entries.iter().enumerate() {
                let bar_h = if max_count > 0 {
                    (count as f64 / max_count as f64) * chart_h
                } else {
                    0.0
                };
                let x = 40.0 + i as f64 * bar_w;
                let y = chart_h - bar_h + 10.0;
                let color = if code < 300 {
                    "#22c55e"
                } else if code < 400 {
                    "#eab308"
                } else if code < 500 {
                    "#f97316"
                } else {
                    "#ef4444"
                };
                svg.push_str(&format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="2"><title>{code}: {count}</title></rect>"#,
                    x = x,
                    y = y,
                    bw = (bar_w - 4.0).max(4.0),
                    bh = bar_h,
                    color = color,
                    code = code,
                    count = count
                ));
                // Count label above bar
                svg.push_str(&format!(
                    r##"<text x="{x:.1}" y="{y:.1}" font-size="11" fill="#334155" text-anchor="middle" font-weight="600">{count}</text>"##,
                    x = x + (bar_w - 4.0).max(4.0) / 2.0,
                    y = y - 4.0,
                    count = count
                ));
                // Code label below
                svg.push_str(&format!(
                    r##"<text x="{x:.1}" y="{y}" font-size="11" fill="#64748b" text-anchor="middle">{code}</text>"##,
                    x = x + (bar_w - 4.0).max(4.0) / 2.0,
                    y = chart_h + 28.0,
                    code = code
                ));
            }
            svg.push_str(&format!(
                r##"<text x="{}" y="{}" font-size="11" fill="#64748b" text-anchor="middle">Status Code</text>"##,
                40.0 + chart_w / 2.0,
                chart_h + 50.0
            ));
            svg.push_str("</svg>");
            svg
        } else {
            String::from("<p>No data available.</p>")
        };

        // Build table rows
        let mut table_rows = String::new();
        for r in &self.responses {
            let status_class = if r.status_code.is_success() {
                "status-ok"
            } else if r.status_code.is_redirection() {
                "status-redirect"
            } else {
                "status-error"
            };
            table_rows.push_str(&format!(
                "<tr><td class=\"url-cell\"><a href=\"{url}\" target=\"_blank\" rel=\"noopener\">{url}</a></td><td>{time}</td><td>{size}</td><td><span class=\"{cls}\">{code}</span></td></tr>\n",
                url = html_escape(&r.url),
                time = r.response_time.as_millis(),
                size = utils::kb(r.response_size),
                cls = status_class,
                code = r.status_code.as_u16(),
            ));
        }

        // Extract stat values for the summary cards
        let rps = if total_time_secs > 0.0 {
            format!("{:.2}", total_requests as f64 / total_time_secs)
        } else {
            "0".to_string()
        };

        let html = format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Siteprobe Report — {sitemap_url}</title>
<style>
*,*::before,*::after{{box-sizing:border-box}}
body{{margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif;background:#f8fafc;color:#1e293b;line-height:1.6}}
.container{{max-width:1200px;margin:0 auto;padding:24px 16px}}
h1{{font-size:1.5rem;font-weight:700;margin:0 0 4px}}
.subtitle{{color:#64748b;font-size:.875rem;margin:0 0 24px}}
.cards{{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px;margin-bottom:32px}}
.card{{background:#fff;border-radius:10px;padding:16px 20px;box-shadow:0 1px 3px rgba(0,0,0,.08)}}
.card .label{{font-size:.75rem;text-transform:uppercase;letter-spacing:.05em;color:#64748b;margin-bottom:4px}}
.card .value{{font-size:1.35rem;font-weight:700;color:#0f172a}}
.section{{background:#fff;border-radius:10px;padding:24px;box-shadow:0 1px 3px rgba(0,0,0,.08);margin-bottom:24px}}
.section h2{{margin:0 0 16px;font-size:1.1rem;font-weight:600}}
.charts{{display:grid;grid-template-columns:1fr 1fr;gap:24px;margin-bottom:24px}}
@media(max-width:768px){{.charts{{grid-template-columns:1fr}}}}
table{{width:100%;border-collapse:collapse;font-size:.85rem}}
th{{position:sticky;top:0;background:#f1f5f9;text-align:left;padding:10px 12px;font-weight:600;cursor:pointer;user-select:none;border-bottom:2px solid #e2e8f0}}
th:hover{{background:#e2e8f0}}
th::after{{content:'';display:inline-block;width:0;height:0;margin-left:6px;vertical-align:middle}}
th.sort-asc::after{{border-left:4px solid transparent;border-right:4px solid transparent;border-bottom:5px solid #475569}}
th.sort-desc::after{{border-left:4px solid transparent;border-right:4px solid transparent;border-top:5px solid #475569}}
td{{padding:8px 12px;border-bottom:1px solid #f1f5f9}}
tr:hover td{{background:#f8fafc}}
.url-cell{{max-width:500px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}
.url-cell a{{color:#4f46e5;text-decoration:none}}
.url-cell a:hover{{text-decoration:underline}}
.status-ok{{color:#16a34a;font-weight:600}}
.status-redirect{{color:#ca8a04;font-weight:600}}
.status-error{{color:#dc2626;font-weight:600}}
.stats-grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:8px 24px}}
.stat-row{{display:flex;justify-content:space-between;padding:6px 0;border-bottom:1px solid #f1f5f9}}
.stat-label{{color:#64748b;font-size:.85rem}}
.stat-value{{font-weight:600;font-size:.85rem}}
footer{{text-align:center;color:#94a3b8;font-size:.75rem;padding:24px 0}}
</style>
</head>
<body>
<div class="container">
<h1>Siteprobe Report</h1>
<p class="subtitle">{sitemap_url} &mdash; {elapsed}</p>

<div class="cards">
  <div class="card"><div class="label">Total Requests</div><div class="value">{total}</div></div>
  <div class="card"><div class="label">Requests/sec</div><div class="value">{rps}</div></div>
  <div class="card"><div class="label">Elapsed Time</div><div class="value">{elapsed}</div></div>
  <div class="card"><div class="label">Concurrency</div><div class="value">{concurrency}</div></div>
  <div class="card"><div class="label">Success Rate</div><div class="value">{success_rate}</div></div>
  <div class="card"><div class="label">Error Rate</div><div class="value">{error_rate}</div></div>
</div>

<div class="section">
<h2>Response Time Statistics</h2>
<div class="stats-grid">
{response_time_stats}
</div>
</div>

<div class="section">
<h2>Performance Statistics</h2>
<div class="stats-grid">
{performance_stats}
</div>
</div>

<div class="charts">
  <div class="section">
    <h2>Response Time Distribution</h2>
    {histogram_svg}
  </div>
  <div class="section">
    <h2>Status Code Breakdown</h2>
    {status_svg}
  </div>
</div>

<div class="section">
<h2>All Responses ({total})</h2>
<div style="overflow-x:auto">
<table id="responses">
<thead>
<tr>
  <th data-col="0">URL</th>
  <th data-col="1">Time (ms)</th>
  <th data-col="2">Size</th>
  <th data-col="3">Status</th>
</tr>
</thead>
<tbody>
{table_rows}
</tbody>
</table>
</div>
</div>

<footer>Generated by Siteprobe {version}</footer>
</div>
<script>
(function(){{
  const table=document.getElementById('responses');
  const headers=table.querySelectorAll('th');
  let sortCol=-1,sortAsc=true;
  headers.forEach(th=>{{
    th.addEventListener('click',function(){{
      const col=+this.dataset.col;
      if(sortCol===col)sortAsc=!sortAsc;else{{sortCol=col;sortAsc=true}}
      headers.forEach(h=>h.classList.remove('sort-asc','sort-desc'));
      this.classList.add(sortAsc?'sort-asc':'sort-desc');
      const tbody=table.querySelector('tbody');
      const rows=Array.from(tbody.querySelectorAll('tr'));
      rows.sort((a,b)=>{{
        let av=a.children[col].textContent.trim();
        let bv=b.children[col].textContent.trim();
        if(col===1||col===3){{av=parseFloat(av)||0;bv=parseFloat(bv)||0}}
        if(av<bv)return sortAsc?-1:1;
        if(av>bv)return sortAsc?1:-1;
        return 0;
      }});
      rows.forEach(r=>tbody.appendChild(r));
    }});
  }});
}})();
</script>
</body>
</html>"##,
            sitemap_url = html_escape(&self.sitemap_url),
            elapsed = format!("{:.2?}", self.total_time),
            total = total_requests,
            rps = rps,
            concurrency = self.concurrency_limit,
            success_rate = stats.status_code.0.iter().find(|e| e.json_label == "successRatePercentage").map(|e| e.value.clone()).unwrap_or_default(),
            error_rate = stats.status_code.0.iter().find(|e| e.json_label == "errorRatePercentage").map(|e| e.value.clone()).unwrap_or_default(),
            response_time_stats = stats.response_time.0.iter().map(|e| format!(
                r#"<div class="stat-row"><span class="stat-label">{}</span><span class="stat-value">{}</span></div>"#,
                html_escape(e.label), html_escape(&e.value)
            )).collect::<Vec<_>>().join("\n"),
            performance_stats = stats.performance.0.iter().map(|e| format!(
                r#"<div class="stat-row"><span class="stat-label">{}</span><span class="stat-value">{}</span></div>"#,
                html_escape(e.label), html_escape(&e.value)
            )).collect::<Vec<_>>().join("\n"),
            histogram_svg = histogram_svg,
            status_svg = status_svg,
            table_rows = table_rows,
            version = env!("CARGO_PKG_VERSION"),
        );

        let mut file = File::create(report_path)?;
        file.write_all(html.as_bytes())?;

        if !options.json {
            println!(
                "\n🌐 The HTML report was written to {}",
                style(report_path.display()).underlined().cyan()
            );
        }

        Ok(())
    }

    /// Determines the appropriate process exit code based on response results.
    ///
    /// - `0` — All URLs returned 2xx (success).
    /// - `1` — One or more URLs returned 4xx/5xx (errors). Takes priority over slow.
    /// - `2` — One or more URLs exceeded the slow threshold (when `--slow-threshold` is set).
    pub fn exit_code(&self, slow_threshold: Option<f64>) -> ExitCode {
        let has_errors = self
            .responses
            .iter()
            .any(|r| r.status_code.is_client_error() || r.status_code.is_server_error());

        if has_errors {
            return ExitCode::from(1);
        }

        if let Some(threshold) = slow_threshold {
            let has_slow = self
                .responses
                .iter()
                .any(|r| r.response_time.as_secs_f64() > threshold);
            if has_slow {
                return ExitCode::from(2);
            }
        }

        ExitCode::SUCCESS
    }

    // === Statistics ==============================================================================

    fn generate_statistics(&self, slow_threshold: Option<f64>) -> Statistics {
        let report = &self;
        let total_requests = report.responses.len();
        let total_time_secs = report.total_time.as_secs_f64();

        let response_times: Vec<Duration> =
            report.responses.iter().map(|r| r.response_time).collect();
        let response_sizes: Vec<usize> = report.responses.iter().map(|r| r.response_size).collect();

        let avg_response_time = if total_requests > 0 {
            response_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / total_requests as f64
        } else {
            0.0
        };
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

        let variance = if total_requests > 0 {
            response_times
                .iter()
                .map(|t| (t.as_secs_f64() - avg_response_time).powi(2))
                .sum::<f64>()
                / total_requests as f64
        } else {
            0.0
        };
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

        let success_rate = if total_requests > 0 {
            (success_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        let error_rate = if total_requests > 0 {
            (error_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        let redirect_rate = if total_requests > 0 {
            (redirect_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        let slow_request_percentage = if total_requests > 0 {
            (slow_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let avg_response_size = if total_requests > 0 {
            response_sizes.iter().sum::<usize>() / total_requests
        } else {
            0
        };
        let min_response_size = response_sizes.iter().copied().min();
        let max_response_size = response_sizes.iter().copied().max();

        Statistics {
            response_time: Metrics(vec![
                Entry {
                    label: "⏰ Average Response Time",
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
                    label: "📏 P90 Response Time",
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
                    label: "📊 Standard Deviation",
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
                    label: "📊 Slow Request Percentage",
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
