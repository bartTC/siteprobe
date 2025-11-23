use once_cell::sync::Lazy;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
use prettytable::{Cell, Row, Table};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;

/// # Table Visualization Utility
///
/// ## Overview
///
/// This Rust file provides a utility for rendering textual tables using the `prettytable` crate.
/// It combines reusable table formatting (powered by `once_cell` for lazy initialization) with a
/// structure for managing labeled key-value pairs, encapsulated in the `Metrics` struct. The main
/// goal of this utility is to create a clean and visually appealing tabular representation of data,
/// which can be leveraged in CLI tools or logging outputs.
///
/// ## Usage
///
/// 1. **Define Metrics**:
///    Populate a list of key-value pairs using `LabeledValue` entries.
///
/// 2. **Convert to Table**:
///    Use the `Metrics::build_table` method to generate the table.
///
/// ### Example:
///
/// ```rust,ignore
/// use your_module_name::{LabeledValue, Metrics};
///
/// let metrics = Metrics(vec![
///     LabeledValue { label: "Key1", value: "Value1".to_string() },
///     LabeledValue { label: "Key2", value: "Value2".to_string() },
/// ]);
///
/// println!("{}", metrics.build_table());
/// ```
///
/// ### Output:
/// ```text
/// ┌──────────┬──────────┐
/// │  Key1    │  Value1  │
/// │  Key2    │  Value2  │
/// └──────────┴──────────┘
/// ```
///
/// ## Dependencies
///
/// - **once_cell**: Used for the lazy initialization of the formatting configuration.
/// - **prettytable**: Provides functionalities for defining table formats and rendering tabular data.
///
///
static TABLE_FORMAT: Lazy<TableFormat> = Lazy::new(|| {
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

pub static CLEAN_FORMAT: Lazy<TableFormat> =
    Lazy::new(|| FormatBuilder::new().padding(0, 3).build());

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
