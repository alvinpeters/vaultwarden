use std::fmt::{Display, Formatter};
#[cfg(fdb)]
use foundationdb::{FdbBindingError, FdbError};
#[cfg(rdb)]
use rocksdb::Error as RdbError;
use thiserror::Error;
use crate::new_db::DbConnType;

#[derive(Error, Debug)]
pub enum DbConnError {
    #[error("c")]
    DbDisabled(DbConnType, String),
    #[error("couldn't establish connection with this string: {0}")]
    EstablishFail(String),
    #[error("failed to start: {0}")]
    StartError(String),
    #[error("failed to stop: {0}")]
    StopError(String),
    #[cfg(fdb)]
    #[error("other FoundationDB error: {0}")]
    FdbError(#[from] FdbError),
    #[cfg(rdb)]
    #[error("other RocksDB error: {0}")]
    RdbError(#[from] RdbError)
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("failed to serialize: {0}")]
    SerializationError(String),
    #[cfg(fdb)]
    #[error("FoundationDB failed: {0}")]
    FdbTrxFailed(#[from] FdbBindingError),
    #[cfg(rdb)]
    #[error("RocksDB failed: {0}")]
    RdbTrxFailed(#[from] RdbError)
}

#[derive(Error, Debug)]
pub(crate) struct TypeConversionError {
    from_type_to_string: String,
    from_type: &'static str,
    to_type: &'static str,
}

impl TypeConversionError {
    pub(crate) fn new_to_string<From, To>(val: &From) -> Self
    where
        From: ToString + ?Sized,
        To: ?Sized
    {
        Self {
            from_type_to_string: val.to_string(),
            from_type: std::any::type_name::<From>(),
            to_type: std::any::type_name::<To>(),
        }
    }

    pub(crate) fn new_from_bytes<To>(val: &[u8]) -> Self
    where
        To: ?Sized
    {
        // This might be costly but this is only called on error anyways
        let from_type_to_string = if val.is_empty() {
            "empty byte slice".to_string()
        } else if val.len() == 1 {
            format!("1 byte [{:#04x}]", val[0])
        } else {
            let mut bytes_string = format!("{} bytes [{:#04x}", val.len(), val[0]);
            let max = 10;
            let iter_until = if val.len() >= 2 && val.len() < max {
                val.len() - 1
            } else {
                max
            };
            // If I'm not stupid, this will never go out of bounds
            for i in 2..iter_until {
                bytes_string += &format!(", {:#04x}", val[i])
            }
            if iter_until >= max {
                bytes_string += ", ... "
            }

            bytes_string + "]"
        };

        Self {
            from_type_to_string,
            from_type: std::any::type_name::<&[u8]>(),
            to_type: std::any::type_name::<To>(),
        }
    }
}

impl Display for TypeConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to convert from {} (value: {}) to {}", self.from_type, self.from_type_to_string, self.to_type)
    }
}