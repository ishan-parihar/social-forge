// ─── DB Type Newtypes ─────────────────────────────────────────
//
// SQLite has no native timestamptz or UUID type. Timestamps live as
// INTEGER epoch seconds and UUIDs as TEXT, which means the plain
// `chrono::DateTime<Utc>` / `Uuid` decoders in sqlx cannot be used
// against these columns directly.
//
// `EpochUtc` is the seam: it reads and writes an INTEGER epoch while
// behaving like a `DateTime<Utc>` at the call site, so query modules
// bind `EpochUtc::from(dt)` and read `EpochUtc` back out of a row
// without caring that the storage is epoch seconds.

use std::fmt;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Type};

/// A UTC timestamp stored as an SQLite INTEGER (seconds since the Unix
/// epoch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochUtc(pub DateTime<Utc>);

impl EpochUtc {
    /// Current time, for defaults and test fixtures.
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

// ── Conversions ─────────────────────────────────────────────

impl From<DateTime<Utc>> for EpochUtc {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<EpochUtc> for DateTime<Utc> {
    fn from(e: EpochUtc) -> Self {
        e.0
    }
}

impl AsRef<DateTime<Utc>> for EpochUtc {
    fn as_ref(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl fmt::Display for EpochUtc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ── sqlx plumbing ───────────────────────────────────────────

impl Type<Sqlite> for EpochUtc {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <i64 as Type<Sqlite>>::compatible(ty)
    }
}

impl<'q> Encode<'q, Sqlite> for EpochUtc {
    fn encode_by_ref(
        &self,
        args: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <i64 as Encode<Sqlite>>::encode_by_ref(&self.0.timestamp(), args)
    }
}

impl<'r> Decode<'r, Sqlite> for EpochUtc {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let secs = <i64 as Decode<Sqlite>>::decode(value)?;
        DateTime::from_timestamp(secs, 0)
            .map(Self)
            .ok_or_else(|| format!("epoch second {secs} is out of range for a UTC timestamp").into())
    }
}

#[cfg(test)]
mod tests {
    use super::EpochUtc;
    use chrono::{DateTime, TimeZone};

    #[test]
    fn round_trips_through_the_inner_datetime() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 3, 14, 15, 9, 26).unwrap();
        let epoch = EpochUtc::from(dt);
        assert_eq!(DateTime::from(epoch), dt);
    }

    #[test]
    fn encodes_as_unix_epoch_seconds() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 3, 14, 15, 9, 26).unwrap();
        assert_eq!(EpochUtc::from(dt).0.timestamp(), dt.timestamp());
    }

    #[test]
    fn the_unix_epoch_round_trips() {
        let epoch = EpochUtc::from(chrono::DateTime::UNIX_EPOCH);
        assert_eq!(epoch.0.timestamp(), 0);
        assert_eq!(DateTime::from(epoch), chrono::DateTime::UNIX_EPOCH);
    }

    #[test]
    fn sub_second_precision_survives_construction_but_is_dropped_on_encode() {
        let dt = chrono::Utc
            .with_ymd_and_hms(2026, 3, 14, 15, 9, 26)
            .unwrap()
            + chrono::Duration::milliseconds(750);
        let epoch = EpochUtc::from(dt);

        // The newtype wraps the DateTime verbatim — no silent mutation.
        assert_eq!(epoch.0, dt);
        assert_eq!(epoch.0.timestamp_subsec_millis(), 750);

        // `Encode` sends `.timestamp()`, so the fractional part is what
        // the INTEGER column actually stores, and a decode of that column
        // comes back truncated to the second.
        assert_eq!(epoch.0.timestamp(), dt.timestamp());
        assert_eq!(DateTime::from(epoch).timestamp(), dt.timestamp());
    }
}

