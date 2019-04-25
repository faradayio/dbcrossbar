//! A driver for working with Postgres.

// See https://github.com/diesel-rs/diesel/issues/1785
#![allow(missing_docs, proc_macro_derive_resolution_fallback)]

use failure::Fail;
use native_tls::TlsConnector;
use std::{
    fmt,
    str::{self, FromStr},
};
pub use tokio_postgres::Client;
use tokio_postgres::Config;
use tokio_postgres_native_tls::MakeTlsConnector;

use crate::common::*;

pub mod citus;
mod csv_to_binary;
mod local_data;
mod schema;
mod write_local_data;

use self::local_data::local_data_helper;
use self::write_local_data::write_local_data_helper;

/// Connect to the database, using SSL if possible.
async fn connect(ctx: Context, url: Url) -> Result<Client> {
    let mut base_url = url.clone();
    base_url.set_fragment(None);

    // Build a basic config from our URL args.
    let config = Config::from_str(base_url.as_str())
        .context("could not configure PostgreSQL connection")?;
    trace!(ctx.log(), "PostgreSQL connection config: {:?}", config);
    let tls_connector = TlsConnector::builder()
        .build()
        .context("could not build PostgreSQL TLS connector")?;
    let (client, connection) =
        await!(config.connect(MakeTlsConnector::new(tls_connector)))
        .context("could not connect to PostgreSQL")?;

    // The docs say we need to run this connection object in the background.
    ctx.spawn_worker(connection.map_err(|e| -> Error {
        e.context("error on PostgreSQL connection").into()
    }));

    Ok(client)
}

/// URL scheme for `PostgresLocator`.
pub(crate) const POSTGRES_SCHEME: &str = "postgres:";

/// A Postgres database URL and a table name.
///
/// This is the central point of access for talking to a running PostgreSQL
/// database.
#[derive(Debug)]
pub struct PostgresLocator {
    url: Url,
    table_name: String,
}

impl fmt::Display for PostgresLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Merge our table name back into our URL.
        let mut full_url = self.url.clone();
        full_url.set_fragment(Some(&self.table_name));
        full_url.fmt(f)
    }
}

impl FromStr for PostgresLocator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut url: Url = s.parse::<Url>().context("cannot parse Postgres URL")?;
        if url.scheme() != &POSTGRES_SCHEME[..POSTGRES_SCHEME.len() - 1] {
            Err(format_err!("expected URL scheme postgres: {:?}", s))
        } else {
            // Extract table name from URL.
            let table_name = url
                .fragment()
                .ok_or_else(|| {
                    format_err!("{} needs to be followed by #table_name", url)
                })?
                .to_owned();
            url.set_fragment(None);
            Ok(PostgresLocator { url, table_name })
        }
    }
}

impl Locator for PostgresLocator {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self, _ctx: &Context) -> Result<Option<Table>> {
        Ok(Some(schema::fetch_from_url(&self.url, &self.table_name)?))
    }

    fn local_data(
        &self,
        ctx: Context,
        schema: Table,
        _temporary_storage: TemporaryStorage,
    ) -> BoxFuture<Option<BoxStream<CsvStream>>> {
        local_data_helper(ctx, self.url.clone(), self.table_name.clone(), schema)
            .into_boxed()
    }

    fn write_local_data(
        &self,
        ctx: Context,
        schema: Table,
        data: BoxStream<CsvStream>,
        _temporary_storage: TemporaryStorage,
        if_exists: IfExists,
    ) -> BoxFuture<BoxStream<BoxFuture<()>>> {
        write_local_data_helper(
            ctx,
            self.url.clone(),
            self.table_name.clone(),
            schema,
            data,
            if_exists,
        )
        .into_boxed()
    }
}
