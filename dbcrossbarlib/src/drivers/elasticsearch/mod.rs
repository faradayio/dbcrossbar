//! A driver for working with Elasticsearch.

use crate::common::*;
use core::fmt;
use std::str::FromStr;

mod count;
mod data_type;
mod field;

pub(crate) use self::data_type::EsDataType;
pub(crate) use self::field::EsField;

/// A Postgres database URL and a table name.
///
/// This is the central point of access for talking to a running PostgreSQL
/// database.
#[derive(Clone, Debug)]
pub struct ElasticsearchLocator {
    url: UrlWithHiddenPassword,
    index: IndexName,
}

impl fmt::Display for ElasticsearchLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.url.fmt(f)
    }
}

/// A PostgreSQL table name, including a possible scheme (i.e., a namespace).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexName {
    index: String,
}

impl FromStr for IndexName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(IndexName {
            index: s.to_owned(),
        })
    }
}

impl FromStr for ElasticsearchLocator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let url: Url = s.parse::<Url>().context("cannot parse Elasticsearch URL")?;
        if url.scheme() != &Self::scheme()[..Self::scheme().len() - 1] {
            Err(format_err!("expected URL scheme elasticsearch: {:?}", s))
        } else {
            // Extract index name from URL.
            let index_name = url.path()[1..].parse::<IndexName>()?;

            // TODO: http vs https
            let scheme = "http";
            let port = url.port().unwrap_or(9200);

            let host = url.host().unwrap();
            let new_url =
                Url::parse(&format!("{}://{}:{}", scheme, host, port)).unwrap();
            let url = UrlWithHiddenPassword::new(new_url);
            Ok(ElasticsearchLocator {
                url,
                index: index_name,
            })
        }
    }
}

#[test]
fn from_str_parses_schemas() {
    let examples = &[
        ("elasticsearch://user:pass@host/db", "http", 9200, "db"),
        ("elasticsearch://user:pass@host:443/db", "http", 443, "db"),
    ];
    for &(url, scheme, port, index) in examples {
        let loc = ElasticsearchLocator::from_str(url).unwrap();
        assert_eq!(loc.url.with_password().scheme(), scheme);
        assert_eq!(loc.index, index.parse::<IndexName>().unwrap(),);
        assert_eq!(loc.url.with_password().port().unwrap(), port);
        assert_eq!(loc.url.with_password().path(), "/");
    }
}

impl Locator for ElasticsearchLocator {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self, _ctx: Context) -> BoxFuture<Option<Table>> {
        let _source = self.to_owned();

        async move {
            Ok(Some(Table {
                name: "huh".to_string(),
                columns: Vec::new(),
            }))
        }
        .boxed()
    }

    fn count(
        &self,
        ctx: Context,
        shared_args: SharedArguments<Unverified>,
        source_args: SourceArguments<Unverified>,
    ) -> BoxFuture<usize> {
        count::count_helper(ctx, self.to_owned(), shared_args, source_args).boxed()
    }

    fn local_data(
        &self,
        _ctx: Context,
        _shared_args: SharedArguments<Unverified>,
        _source_args: SourceArguments<Unverified>,
    ) -> BoxFuture<Option<BoxStream<CsvStream>>> {
        unimplemented!();
    }

    fn write_local_data(
        &self,
        _ctx: Context,
        _data: BoxStream<CsvStream>,
        _shared_args: SharedArguments<Unverified>,
        _dest_args: DestinationArguments<Unverified>,
    ) -> BoxFuture<BoxStream<BoxFuture<BoxLocator>>> {
        unimplemented!();
    }
}

impl LocatorStatic for ElasticsearchLocator {
    fn scheme() -> &'static str {
        "elasticsearch:"
    }

    fn features() -> Features {
        Features {
            locator: LocatorFeatures::Schema
                | LocatorFeatures::LocalData
                | LocatorFeatures::WriteLocalData
                | LocatorFeatures::Count,
            write_schema_if_exists: IfExistsFeatures::Error.into(),
            source_args: SourceArgumentsFeatures::WhereClause.into(),
            dest_args: EnumSet::empty(),
            dest_if_exists: IfExistsFeatures::Upsert.into(),
            _placeholder: (),
        }
    }
}

/// An Elasticsearch index declaration.
///
/// This is marked as `pub` and not `pub(crate)` because of a limitation of the
/// `peg` crate, which can only declare regular `pub` functions, which aren't
/// allowed to expose `pub(crate)` types. But we don't actually want to export
/// this outside of our crate, so we mark it `pub` here but take care to not
/// export it from a `pub` module anywhere.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EsCreateIndex {
    /// The name of the table.
    pub(crate) name: IndexName,
    /// The columns in the table.
    pub(crate) fields: Vec<EsField>,
}
