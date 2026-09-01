use prettytable::{Cell, Row, Table};
use serde_json::json;
use siteprobe::metrics::{CLEAN_FORMAT, Entry, Metrics};

#[test]
fn test_visual_alignment() {
    let metrics = Metrics(vec![
        Entry {
            label: "⏰ Average Response Time",
            value: "100ms".to_string(),
            json_label: "avgMs",
            json_value: json!(100),
        },
        Entry {
            label: "🔷 Median Response Time",
            value: "100ms".to_string(),
            json_label: "medianMs",
            json_value: json!(100),
        },
        Entry {
            label: "🐇 Min Response Time",
            value: "100ms".to_string(),
            json_label: "minMs",
            json_value: json!(100),
        },
        Entry {
            label: "🐌 Max Response Time",
            value: "100ms".to_string(),
            json_label: "maxMs",
            json_value: json!(100),
        },
        Entry {
            label: "📏 P90 Response Time",
            value: "100ms".to_string(),
            json_label: "p90Ms",
            json_value: json!(100),
        },
        Entry {
            label: "🎯 P95 Response Time",
            value: "100ms".to_string(),
            json_label: "p95Ms",
            json_value: json!(100),
        },
        Entry {
            label: "🚀 P99 Response Time",
            value: "100ms".to_string(),
            json_label: "p99Ms",
            json_value: json!(100),
        },
        Entry {
            label: "📊 Standard Deviation",
            value: "100ms".to_string(),
            json_label: "stdDevMs",
            json_value: json!(100),
        },
        Entry {
            label: "✅ Success Rate",
            value: "100%".to_string(),
            json_label: "successRate",
            json_value: json!(100),
        },
        Entry {
            label: "🚨 Error Rate",
            value: "0%".to_string(),
            json_label: "errorRate",
            json_value: json!(0),
        },
        Entry {
            label: "🔄 Redirect Rate",
            value: "0%".to_string(),
            json_label: "redirectRate",
            json_value: json!(0),
        },
        Entry {
            label: "⚡️ Total Requests Processed",
            value: "100".to_string(),
            json_label: "totalRequests",
            json_value: json!(100),
        },
        Entry {
            label: "⏳ Requests Per Second (RPS)",
            value: "10/sec".to_string(),
            json_label: "rps",
            json_value: json!(10),
        },
        Entry {
            label: "📊 Slow Request Percentage",
            value: "0%".to_string(),
            json_label: "slowRequestPercentage",
            json_value: json!(0),
        },
        Entry {
            label: "📦 Average Response Size",
            value: "1KB".to_string(),
            json_label: "avgResponseSize",
            json_value: json!(1024),
        },
        Entry {
            label: "🔹 Min Response Size",
            value: "1KB".to_string(),
            json_label: "minResponseSize",
            json_value: json!(1024),
        },
        Entry {
            label: "🔺 Max Response Size",
            value: "1KB".to_string(),
            json_label: "maxResponseSize",
            json_value: json!(1024),
        },
    ]);

    let mut table = Table::new();
    table.set_format(*CLEAN_FORMAT);
    table.add_row(Row::new(vec![Cell::new(metrics.build_table().as_str())]));
    println!("\n{}", table);
}
