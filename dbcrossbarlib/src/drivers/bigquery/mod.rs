//! Driver for working with BigQuery schemas.

use lazy_static::lazy_static;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use std::{fmt, fs::File, iter, process::Command, str::FromStr};
use tempdir::TempDir;
use tokio_process::CommandExt;

use crate::common::*;
use crate::drivers::gs::GsLocator;

mod write_schema;

/// URL scheme for `BigQueryLocator`.
pub(crate) const BIGQUERY_SCHEME: &str = "bigquery:";

/// A locator for a BigQuery table.
#[derive(Debug, Clone)]
pub struct BigQueryLocator {
    /// The name of the Google Cloud project.
    pub project: String,
    /// The BigQuery dataset.
    pub dataset: String,
    /// The table.
    pub table: String,
}

impl BigQueryLocator {
    /// Return the full name of table pointed to by this locator.
    fn to_full_table_name(&self) -> String {
        format!("{}:{}.{}", self.project, self.dataset, self.table)
    }

    /// Construct a temporary table name based on our regular table name.
    ///
    /// TODO: We place this in the same data set as the original table, which
    /// may cause problems for people using wildcard table names. I think we may
    /// want some way for users to specify a temporary table name.
    fn temp_table_name(&self) -> String {
        let mut rng = thread_rng();
        let tag = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(5)
            .collect::<String>();
        format!(
            "{}:{}.temp_{}_{}",
            self.project, self.dataset, self.table, tag
        )
    }
}

impl fmt::Display for BigQueryLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "bigquery:{}:{}.{}",
            self.project, self.dataset, self.table
        )
    }
}

impl FromStr for BigQueryLocator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        lazy_static! {
            static ref RE: Regex = Regex::new("^bigquery:([^:.]+):([^:.]+).([^:.]+)$")
                .expect("could not parse built-in regex");
        }
        let cap = RE
            .captures(s)
            .ok_or_else(|| format_err!("could not parse locator: {:?}", s))?;
        let (project, dataset, table) = (&cap[1], &cap[2], &cap[3]);
        Ok(BigQueryLocator {
            project: project.to_string(),
            dataset: dataset.to_string(),
            table: table.to_string(),
        })
    }
}

impl Locator for BigQueryLocator {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn supports_write_remote_data(&self, source: &dyn Locator) -> bool {
        // We can only do `write_remote_data` if `source` is a `GsLocator`.
        // Otherwise, we need to do `write_local_data` like normal.
        source.as_any().is::<GsLocator>()
    }

    fn write_remote_data(
        &self,
        ctx: Context,
        schema: Table,
        source: BoxLocator,
        if_exists: IfExists,
    ) -> BoxFuture<()> {
        write_remote_data_helper(self.to_owned(), ctx, schema, source, if_exists)
            .into_boxed()
    }
}

async fn write_remote_data_helper(
    dest: BigQueryLocator,
    ctx: Context,
    schema: Table,
    source: BoxLocator,
    if_exists: IfExists,
) -> Result<()> {
    // Convert the source locator into the underlying `gs://` URL. This is a bit
    // fiddly because we're downcasting `source` and relying on knowledge about
    // the `GsLocator` type, and Rust doesn't make that especially easy.
    let mut source_url = source
        .as_any()
        .downcast_ref::<GsLocator>()
        .ok_or_else(|| format_err!("not a gs:// locator: {}", source))?
        .as_url()
        .to_owned();

    // Decide if we need to use a temp table.
    let (use_temp, initial_table) = if write_schema::need_import_sql(&schema) {
        (true, dest.temp_table_name())
    } else {
        (false, dest.to_full_table_name())
    };

    // If our URL looks like a directory, add a glob.
    //
    // TODO: Is this the right way to default this? Or should we make users
    // always specify `*.csv`? This should probably be part of some larger
    // `dbcrossbar` property. Elsewhere, we're trying to default to adding
    // `**/*.csv`, but that's not supported by BigQuery.
    if source_url.as_str().ends_with('/') {
        source_url = source_url.join("*.csv")?;
    }

    // Write our schema to a temp file. This actually needs to be somewhere on
    // disk, and `bq` uses various hueristics to detect that it's a file
    // containing a schema, and not just a string with schema text. (Note this
    // code is synchronous, but that's not a huge deal.)
    //
    // We use `use_temp` to decide whether to generate the final schema or a
    // temporary schema that we'll fix later.
    let tmp_dir = TempDir::new("bq_load")?;
    let schema_path = tmp_dir.path().join("schema.json");
    let mut schema_file = File::create(&schema_path)?;
    write_schema::write_json(&mut schema_file, &schema, use_temp)?;

    // Build and run a `bq load` command.
    let child = Command::new("bq")
        // These arguments can all be represented as UTF-8 `&str`.
        .args(&[
            "load",
            "--skip_leading_rows=1",
            &initial_table,
            source_url.as_str(),
        ])
        // This argument is a path, and so it might contain non-UTF-8
        // characters. We pass it separately so Rust can do the right thing.
        .arg(&schema_path)
        .spawn_async()
        .map_err(|e| format_err!("error starting `bq load`: {}", e))?;
    let status =
        await!(child).map_err(|e| format_err!("error running `bq load`: {}", e))?;
    if !status.success() {
        return Err(format_err!("`bq load` failed with {}", status));
    }

    // TODO: Run the update SQL to build the final table (if needed).
    // TODO: if_exists

    Ok(())
}

/// URL scheme for `PostgresSqlLocator`.
pub(crate) const BIGQUERY_SCHEMA_SCHEME: &str = "bigquery-schema:";

/// A JSON file containing BigQuery table schema.
#[derive(Debug)]
pub struct BigQuerySchemaLocator {
    path: PathOrStdio,
}

impl fmt::Display for BigQuerySchemaLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.fmt_locator_helper(BIGQUERY_SCHEMA_SCHEME, f)
    }
}

impl FromStr for BigQuerySchemaLocator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let path = PathOrStdio::from_str_locator_helper(BIGQUERY_SCHEMA_SCHEME, s)?;
        Ok(BigQuerySchemaLocator { path })
    }
}

impl Locator for BigQuerySchemaLocator {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn write_schema(
        &self,
        ctx: &Context,
        table: &Table,
        if_exists: IfExists,
    ) -> Result<()> {
        let mut f = self.path.create_sync(ctx, if_exists)?;
        write_schema::write_json(&mut f, table, false)
    }
}
