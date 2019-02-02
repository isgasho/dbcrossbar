//! Support for reading data from a PostgreSQL table.

use log::{error, warn};
use pipe::{pipe, PipeReader};
use std::{
    io::{self, Read, Write},
    thread,
};
use url::Url;

use super::connect;
use crate::schema::{DataType, Table};
use crate::{CsvStream, Result};

/// A `Read` implementation for the stream produced by `copy_out_table`.
pub struct CopyOutTable {
    /// A reader, which is supplied with data by a background thread.
    rdr: PipeReader,
    /// A handle which allows us to wait for our background thread to finish.
    handle: Option<thread::JoinHandle<Result<()>>>,
}

impl CopyOutTable {
    /// Join our background thread (ie, wait for it to finish), taking care to
    /// handle errors correctly.
    fn join_helper(&mut self) -> io::Result<()> {
        match self.handle.take() {
            // We've already joined our background thread once. Technically this
            // can happen, but only if our caller is doing something weird, like
            // retrying reads after an EOF.
            None => {
                warn!("tried to join background I/O thread more than once");
                Ok(())
            }
            // We're joining for the first time.
            Some(handle) => {
                let result = handle.join().expect("background I/O panic");
                match result {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        error!("{}", err);
                        let msg = format!("background I/O error: {}", err);
                        Err(io::Error::new(io::ErrorKind::Other, msg))
                    }
                }
            }
        }
    }
}

impl Read for CopyOutTable {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.rdr.read(buf) {
            // `Ok(0)` implies an end-of-file, so join our background thread.
            Ok(0) => {
                self.join_helper()?;
                Ok(0)
            }
            // Pass normal reads through.
            Ok(n) => Ok(n),
            // Pass errors through, joining our background thread for all except
            // `ErrorKind::Interrupted`, which should be retried.
            Err(err) => {
                if err.kind() != io::ErrorKind::Interrupted {
                    let _ = self.join_helper();
                }
                Err(err)
            }
        }
    }
}

/// Copy the specified table from the database, returning a `CsvStream`.
pub(crate) fn copy_out_table(url: &Url, table: &Table) -> Result<CsvStream> {
    // Generate SQL for query.
    let mut sql_bytes: Vec<u8> = vec![];
    write_select(&mut sql_bytes, table)?;
    let sql = String::from_utf8(sql_bytes).expect("should always be UTF-8");

    // Use `pipe` and a background thread to convert a `Write` to `Read`.
    let url = url.clone();
    let (rdr, mut wtr) = pipe();
    let handle = thread::spawn(move || -> Result<()> {
        let conn = connect(&url)?;
        let stmt = conn.prepare(&sql)?;
        stmt.copy_out(&[], &mut wtr)?;
        Ok(())
    });

    Ok(CsvStream {
        name: table.name.clone(),
        data: Box::new(CopyOutTable {
            rdr,
            handle: Some(handle),
        }),
    })
}

/// Generate a complete `SELECT` statement which outputs the table as CSV,
/// in a format that can likely be imported by other database.
fn write_select(f: &mut dyn Write, table: &Table) -> Result<()> {
    write!(f, "COPY (SELECT ")?;
    write_select_args(f, table)?;
    write!(f, " FROM {:?}", table.name)?;
    write!(f, ") TO STDOUT WITH CSV HEADER")?;
    Ok(())
}

/// Write out a table's column names as `SELECT` arguments.
fn write_select_args(f: &mut dyn Write, table: &Table) -> Result<()> {
    let mut first: bool = true;
    for col in &table.columns {
        if first {
            first = false;
        } else {
            write!(f, ",")?;
        }
        match &col.data_type {
            DataType::Array(_) => {
                write!(f, "array_to_json({:?}) AS {:?}", col.name, col.name)?;
            }
            DataType::GeoJson => {
                // Always transform to SRID 4326.
                write!(
                    f,
                    "ST_AsGeoJSON(ST_Transform({:?}, 4326)) AS {:?}",
                    col.name, col.name,
                )?;
            }
            _ => write!(f, "{:?}", col.name)?,
        }
    }
    Ok(())
}