//! Drivers for various schema sources and destinations.
//!
//! These APIs are all unstable and not yet standardized.

use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::common::*;
use crate::locator::{LocatorDriver, LocatorDriverWrapper};

pub mod bigml;
pub mod bigquery;
pub mod bigquery_schema;
pub mod bigquery_shared;
pub mod csv;
pub mod dbcrossbar_schema;
pub mod dbcrossbar_ts;
pub mod elasticsearch;
pub mod gs;
pub mod postgres;
pub mod postgres_shared;
pub mod postgres_sql;
pub mod redshift;
pub mod s3;
pub mod shopify;

/// A helper which builds a `Box<dyn LocatorDriver>` for a type implementating
/// `LocatorStatic`.
fn driver<L: LocatorStatic>() -> Box<dyn LocatorDriver> {
    Box::new(LocatorDriverWrapper::<L>::new())
}

lazy_static! {
    /// A list of known drivers, computed the first time we use it and cached.
    static ref KNOWN_DRIVERS: Vec<Box<dyn LocatorDriver>> = vec![
        driver::<bigml::BigMlLocator>(),
        driver::<bigquery::BigQueryLocator>(),
        driver::<bigquery_schema::BigQuerySchemaLocator>(),
        driver::<csv::CsvLocator>(),
        driver::<dbcrossbar_schema::DbcrossbarSchemaLocator>(),
        driver::<dbcrossbar_ts::DbcrossbarTsLocator>(),
        driver::<elasticsearch::ElasticsearchLocator>(),
        driver::<gs::GsLocator>(),
        driver::<postgres::PostgresLocator>(),
        driver::<postgres_sql::PostgresSqlLocator>(),
        driver::<redshift::RedshiftLocator>(),
        driver::<s3::S3Locator>(),
        driver::<shopify::ShopifyLocator>(),
    ];

    /// A hash table of all known drivers, indexed by scheme and computed the
    /// first time we use it.
    static ref KNOWN_DRIVERS_BY_SCHEME: HashMap<&'static str, &'static dyn LocatorDriver> = {
        let mut table = HashMap::new();
        for driver in KNOWN_DRIVERS.iter() {
            table.insert(driver.scheme(), driver.as_ref());
        }
        table
    };
}

/// All known drivers.
pub fn all_drivers() -> &'static [Box<dyn LocatorDriver>] {
    &KNOWN_DRIVERS[..]
}

/// Look up a specifc driver by `Locator` scheme.
pub fn find_driver(
    scheme: &str,
    enable_unstable: bool,
) -> Result<&'static dyn LocatorDriver> {
    KNOWN_DRIVERS_BY_SCHEME
        .get(scheme)
        .copied()
        .filter(|&d| !d.is_unstable() || enable_unstable)
        .ok_or_else(|| format_err!("unknown locator scheme {:?}", scheme))
}
