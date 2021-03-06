//! The `schema` subcommand.

use common_failures::Result;
use dbcrossbarlib::{BoxLocator, Context, IfExists};
use failure::format_err;
use structopt::{self, StructOpt};

/// Schema conversion arguments.
#[derive(Debug, StructOpt)]
pub(crate) struct Opt {
    /// One of `error`, `overrwrite` or `append`.
    #[structopt(long = "if-exists", default_value = "error")]
    if_exists: IfExists,

    /// The input schema.
    from_locator: BoxLocator,

    /// The output schema.
    to_locator: BoxLocator,
}

/// Perform our schema conversion.
pub(crate) async fn run(ctx: Context, opt: Opt) -> Result<()> {
    let schema = opt.from_locator.schema(&ctx)?.ok_or_else(|| {
        format_err!("don't know how to read schema from {}", opt.from_locator)
    })?;
    opt.to_locator.write_schema(&ctx, &schema, opt.if_exists)?;
    Ok(())
}
