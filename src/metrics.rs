//! Table visualization utility for report metrics.
//!
//! A [`Metrics`] value holds a list of labeled [`Entry`] items, where each entry
//! carries a human-readable label/value pair for the text report and a
//! `json_label`/`json_value` pair for the JSON report. [`Metrics::build_table`]
//! renders the entries as a bordered table using the `prettytable` crate, and
//! the [`serde::Serialize`] implementation turns them into a flat
//! `{jsonLabel: value}` JSON object.

use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
use prettytable::{Cell, Row, Table};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

/// A bordered table format with box-drawing characters, used for metric blocks.
static TABLE_FORMAT: LazyLock<TableFormat> = LazyLock::new(|| {
    FormatBuilder::new()
        .column_separator('│')
        .borders('│')
        .separators(&[LinePosition::Top], LineSeparator::new('─', '┬', '┌', '┐'))
        .separators(
            &[LinePosition::Bottom],
            LineSeparator::new('─', '┴', '└', '┘'),
        )
        .padding(1, 1)
        .build()
});

/// A borderless format used to lay out metric tables side by side.
pub static CLEAN_FORMAT: LazyLock<TableFormat> =
    LazyLock::new(|| FormatBuilder::new().padding(0, 3).build());

#[derive(Debug, Serialize)]
pub struct Entry {
    pub label: &'static str,
    pub value: String,
    pub json_label: &'static str,
    pub json_value: Value,
}

#[derive(Debug)]
pub struct Metrics(pub Vec<Entry>);

impl Metrics {
    pub fn build_table(&self) -> String {
        let mut table = Table::new();
        table.set_format(*TABLE_FORMAT);
        for entry in &self.0 {
            table.add_row(Row::new(vec![
                Cell::new(entry.label),
                Cell::new(&entry.value),
            ]));
        }
        table.to_string()
    }
}

impl Serialize for Metrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Metrics entries into a simple HashMap with {jsonLabel: value}
        let map: HashMap<_, _> = self
            .0
            .iter()
            .map(|entry| (entry.json_label, entry.json_value.clone()))
            .collect();
        map.serialize(serializer)
    }
}
