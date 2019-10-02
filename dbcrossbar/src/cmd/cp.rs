//! The `cp` subcommand.

use common_failures::Result;
use dbcrossbarlib::{
    rechunk::rechunk_csvs,
    tokio_glue::{BoxFuture, BoxStream},
    BoxLocator, Context, DestinationArguments, DisplayOutputLocators, DriverArguments,
    IfExists, SharedArguments, SourceArguments, TemporaryStorage,
};
use failure::format_err;
use futures::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use humanize_rs::bytes::Bytes as HumanizedBytes;
use slog::{debug, o};
use structopt::{self, StructOpt};
use tokio::{
    codec::{FramedWrite, LinesCodec},
    io,
    prelude::{stream, Stream},
};

/// Schema conversion arguments.
#[derive(Debug, StructOpt)]
pub(crate) struct Opt {
    /// One of `error`, `overwrite` or `append`.
    #[structopt(long = "if-exists", default_value = "error")]
    if_exists: IfExists,

    /// The schema to use (defaults to input table schema).
    #[structopt(long = "schema")]
    schema: Option<BoxLocator>,

    /// Temporary directories, cloud storage buckets, datasets to use during
    /// transfer (can be repeated).
    #[structopt(long = "temporary")]
    temporaries: Vec<String>,

    /// Specify the approximate size of the CSV streams manipulated by
    /// `dbcrossbar`. This can be used to split a large input into multiple
    /// smaller outputs. Actual data streams may be bigger or smaller depending
    /// on a number of factors. Examples: "100000", "1Gb".
    #[structopt(long = "stream-size")]
    stream_size: Option<HumanizedBytes>, // usize

    /// Pass an extra argument of the form `key=value` to the source driver.
    #[structopt(long = "from-arg")]
    from_args: Vec<String>,

    /// Pass an extra argument of the form `key=value` to the destination
    /// driver.
    #[structopt(long = "to-arg")]
    to_args: Vec<String>,

    /// SQL where clause specifying rows to use.
    #[structopt(long = "where")]
    where_clause: Option<String>,

    /// Display where we wrote our output data.
    #[structopt(long = "display-output-locators")]
    display_output_locators: bool,

    /// The input table.
    from_locator: BoxLocator,

    /// The output table.
    to_locator: BoxLocator,
}

/// Perform our schema conversion.
pub(crate) async fn run(ctx: Context, opt: Opt) -> Result<()> {
    // Figure out what table schema to use.
    let schema = {
        let schema_locator = opt.schema.as_ref().unwrap_or(&opt.from_locator);
        schema_locator.schema(ctx.clone()).await?.ok_or_else(|| {
            format_err!("don't know how to read schema from {}", opt.from_locator)
        })
    }?;

    // Build our shared arguments.
    let temporary_storage = TemporaryStorage::new(opt.temporaries.clone());
    let shared_args = SharedArguments::new(schema, temporary_storage);

    // Build our source arguments.
    let from_args = DriverArguments::from_cli_args(&opt.from_args)?;
    let source_args = SourceArguments::new(from_args, opt.where_clause.clone());

    // Build our destination arguments.
    let to_args = DriverArguments::from_cli_args(&opt.to_args)?;
    let dest_args = DestinationArguments::new(to_args, opt.if_exists);

    // Can we short-circuit this particular copy using special features of the
    // the source and destination, or do we need to pull the data down to the
    // local machine?
    let to_locator = opt.to_locator;
    let from_locator = opt.from_locator;
    let should_use_remote = opt.stream_size.is_none()
        && to_locator.supports_write_remote_data(from_locator.as_ref());
    let dests = if should_use_remote {
        // Build a logging context.
        let ctx = ctx.child(o!(
            "from_locator" => from_locator.to_string(),
            "to_locator" => to_locator.to_string(),
        ));

        // Perform a remote transfer.
        debug!(ctx.log(), "performing remote data transfer");
        let dests = to_locator
            .write_remote_data(ctx, from_locator, shared_args, source_args, dest_args)
            .await?;

        // Convert our list of output locators into a stream.
        Box::new(stream::iter_ok(dests)) as BoxStream<BoxLocator>
    } else {
        // We have to transfer the data via the local machine, so read data from
        // input.
        debug!(ctx.log(), "performing local data transfer");

        let input_ctx = ctx.child(o!("from_locator" => from_locator.to_string()));
        let mut data = from_locator
            .local_data(input_ctx, shared_args.clone(), source_args)
            .await?
            .ok_or_else(|| {
                format_err!("don't know how to read data from {}", from_locator)
            })?;

        // Honor --stream-size if passed.
        if let Some(stream_size) = opt.stream_size {
            let stream_size = stream_size.size();
            data = rechunk_csvs(ctx.clone(), stream_size, data)?;
        }

        // Write data to output.
        let output_ctx = ctx.child(o!("to_locator" => to_locator.to_string()));
        let result_stream = to_locator
            .write_local_data(output_ctx, data, shared_args, dest_args)
            .await?;

        // Consume the stream of futures produced by `write_local_data`, allowing a
        // certain degree of parallelism. This is where all the actual work happens,
        // and this what controls how many "input driver" -> "output driver"
        // connections are running at any given time.
        Box::new(
            result_stream
                // This stream contains std futures, but we need tokio futures for
                // `buffered`.
                .map(|fut: BoxFuture<BoxLocator>| fut.compat())
                // Run up to `parallelism` futures in parallel.
                .buffer_unordered(4),
        ) as BoxStream<BoxLocator>
    };

    // Optionally display `dests`, depending on a combination of
    // `--display-output-locators` and the defaults for `to_locator`.
    let display_output_locators = match (
        opt.display_output_locators,
        to_locator.display_output_locators(),
    ) {
        // The user passed `--display-output-locators`, but displaying them is
        // forbidden (probably because we wrote actual data to standard output).
        (true, DisplayOutputLocators::Never) => {
            return Err(format_err!(
                "cannot use --display-output-locators with {}",
                to_locator
            ))
        }

        // We want to display our actual output locators.
        (true, _) | (false, DisplayOutputLocators::ByDefault) => true,

        // We don't want to display our output locators.
        (false, _) => false,
    };

    // Print our destination
    if display_output_locators {
        // Display our output locators incrementally on standard output using
        // `LinesCodec` to insert newlines.
        let stdout_sink = FramedWrite::new(io::stdout(), LinesCodec::new());
        let (_dests, _stdout_sink) = dests
            .and_then(|dest| {
                let dest_str = dest.to_string();
                if dest_str.contains('\n') || dest_str.contains('\r') {
                    // If we write out this locator, it would be split between
                    // lines, causing an ambiguity for any parsing program.
                    Err(format_err!(
                        "cannot output locator with newline: {:?}",
                        dest_str
                    ))
                } else {
                    Ok(dest_str)
                }
            })
            .forward(stdout_sink)
            .compat()
            // Needed to work around
            // https://users.rust-lang.org/t/solved-one-type-is-more-general-than-the-other-but-they-are-exactly-the-same/25194
            .boxed()
            .await?;
    } else {
        // Just collect our results and ignore
        let dests = dests.collect().compat().boxed().await?;
        debug!(ctx.log(), "destination locators: {:?}", dests);
    }
    Ok(())
}
