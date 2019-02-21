//! A short spike to see if we can generate working `FORMAT BINARY` for
//! PostgreSQL.

use byteorder::{NetworkEndian, WriteBytesExt};
use std::{error::Error, io::{self, prelude::*}, mem::size_of};

type NE = NetworkEndian;

fn main() -> Result<(), Box<Error>> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Header.
    out.write_all(b"PGCOPY\n")?;
    out.write_all(&[0o377])?;
    out.write_all(b"\r\n\0")?;

    // Flags.
    out.write_u32::<NE>(0)?;

    // Header extension area length.
    out.write_u32::<NE>(0)?;

    // Tuple field count.
    out.write_i16::<NE>(6)?;

    // Field: NULL.
    out.write_i32::<NE>(-1)?;

    // Field: Array of Int32.
    // Field: Bool.
    // Field: Date.
    // Field: Decimal.
    // Field: Float32.
    out.write_i32::<NE>(size_of::<f32>() as i32)?;
    out.write_f32::<NE>(32.0)?;

    // Field: Float64.
    out.write_i32::<NE>(size_of::<f64>() as i32)?;
    out.write_f64::<NE>(64.0)?;

    // Field: GeoJson.
    // Field: Int16.
    out.write_i32::<NE>(size_of::<i16>() as i32)?;
    out.write_i16::<NE>(16)?;

    // Field: Int32.
    out.write_i32::<NE>(size_of::<i32>() as i32)?;
    out.write_i32::<NE>(32)?;

    // Field: Int64.
    out.write_i32::<NE>(size_of::<i64>() as i32)?;
    out.write_i64::<NE>(64)?;

    // Field: JSON.
    // Field: Text.
    // Field: Timestamp without time zone.
    // Field: Timestamp with time zone.
    // Field: UUID.

    // File trailer.
    out.write_i16::<NE>(-1)?;

    Ok(())
}
