//! Siteprobe fetches all URLs from a `sitemap.xml`, checks their existence,
//! and generates performance reports.
//!
//! This library interface exists primarily so the binary and the integration
//! tests can share the same modules; it is not a stable public API.

pub mod metrics;
pub mod network;
pub mod options;
pub mod report;
pub mod sitemap;
pub mod storage;
pub mod utils;
