//! Implementation of `count`, but as a real `async` function.

use crate::common::*;
use crate::drivers::elasticsearch::ElasticsearchLocator;
use crate::{Context, SharedArguments, SourceArguments, Unverified};
use serde::Deserialize;

/// Implementation of `count`, but as a real `async` function.
pub(crate) async fn count_helper(
    _ctx: Context,
    locator: ElasticsearchLocator,
    shared_args: SharedArguments<Unverified>,
    source_args: SourceArguments<Unverified>,
) -> Result<usize> {
    let _shared_args = shared_args.verify(ElasticsearchLocator::features())?;
    let _source_args = source_args.verify(ElasticsearchLocator::features())?;

    // Get the parts of our locator.
    let url = locator.url.clone();
    let index = locator.index.clone();

    // Generate HTTP request for query
    let base = url.with_password().clone();
    let url = base
        .join(&format!("{}/_count", &index.index))
        .context("joining up paths")?;
    let response = reqwest::get(url).await?;
    let count = response.json::<CountResponse>().await?;

    Ok(count.count)
}

/// {"count":747,"_shards":{"total":1,"successful":1,"skipped":0,"failed":0}}%
#[derive(Deserialize)]
struct CountResponse {
    pub count: usize,
    #[serde(rename = "_shards")]
    pub shards: ShardsResponse,
}

#[derive(Deserialize)]
struct ShardsResponse {
    pub total: usize,
    pub successful: usize,
    pub skipped: usize,
    pub failed: usize,
}
