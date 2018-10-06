//! GIS support.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    str::FromStr,
};

/// An European Petroleum Survey Group spatial reference identifier. This
/// identifies a geographic coordinate system used for representing latitude and
/// longitude precisely.
///
/// This is a semi-transparent wrapper around an integer, just to make it clear
/// why we're passing integers around and to provie a convenient place to hang
/// methods like `default`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EpsgSrid(pub u32);

impl Default for EpsgSrid {
    /// SRID EPSG:4326 (WGS 84) is the most popular interchange format.
    fn default() -> Self {
        EpsgSrid(4326)
    }
}

impl FromStr for EpsgSrid {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(EpsgSrid(FromStr::from_str(s)?))
    }
}

impl fmt::Display for EpsgSrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for EpsgSrid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        Ok(EpsgSrid(Deserialize::deserialize(deserializer)?))
    }
}

impl Serialize for EpsgSrid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        self.0.serialize(serializer)
    }
}
