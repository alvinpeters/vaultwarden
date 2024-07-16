use std::any::Any;
use bson::{Binary, DateTime};
use chrono::NaiveDateTime;
use rocket::data::N;
use serde::{Deserialize, Serialize};

pub(crate) trait FromDbType<T>: Sized {
    fn from_db_type(value: T) -> Self;
}

pub(crate) trait IntoDbType<T>: Sized {
    fn into_db_type(self) -> T;
}

pub(crate) trait TryFromDbType<T>: Sized {
    fn try_from_db_type(value: T) -> Self;
}

pub(crate) trait TryIntoDbType<T>: Sized {
    fn try_into_db_type(self) -> T;
}

pub(crate) trait FromModelType<T>: Sized {
    fn from_model_type(value: T) -> Self;
}

pub(crate) trait IntoModelType<T>: Sized {
    fn into_model_type(self) -> T;
}

pub(crate) trait FromCompatType<T>: Sized {
    fn from_compat_type(value: T) -> Self;
}

pub(crate) trait IntoCompatType<T>: Sized {
    fn into_compat_type(self) -> T;
}

/// Allows conversion to self. Useful for macros
impl<T> FromCompatType<T> for T where T: Any {
    #[inline(always)]
    fn from_compat_type(value: T) -> Self {
        value
    }
}

impl<T, U> IntoCompatType<U> for T where U: FromCompatType<T> {
    #[inline(always)]
    fn into_compat_type(self) -> U {
        U::from_compat_type(self)
    }
}

/// Converts a naive date time type into BSON-compatible date time with UTC assumed.
impl FromCompatType<NaiveDateTime> for DateTime {
    #[inline(always)]
    fn from_compat_type(value: NaiveDateTime) -> Self {
        DateTime::from_chrono(value.and_utc())
    }
}

/// Converts BSON date time into a naive date time with UTC assumed.
impl FromCompatType<DateTime> for NaiveDateTime {
    #[inline(always)]
    fn from_compat_type(value: DateTime) -> Self {
        value.to_chrono().naive_utc()
    }
}

impl<T, U> IntoModelType<U> for T where U: FromDbType<T> {
    #[inline(always)]
    fn into_model_type(self) -> U {
        U::from_db_type(self)
    }
}

impl<T, U> IntoDbType<U> for T where U: FromModelType<T> {
    #[inline(always)]
    fn into_db_type(self) -> U {
        U::from_model_type(self)
    }
}

#[cfg(serde_doc)]
impl<'de, T> FromDbType<&'de [u8]> for T where T: Deserialize<'de> {
    /// Deserialize a BSON slice but panics without handling any errors.
    ///
    /// # Panics
    ///
    /// This function panics if `bson::from_slice` returns error for:
    ///     - encountering `std::io::Error`,
    ///     - encountering invalid UTF-8 string when deserializing to string,
    ///     - attempting to decode a non-BSON type,
    ///     - input ended early,
    ///     - other deserialization errors.
    fn from_db_type(value: &'de [u8]) -> Self {
        bson::from_slice(value).expect("deserializable BSON slice")
    }
}


#[cfg(serde_doc)]
impl<T> FromModelType<&T> for Vec<u8> where T: Serialize {
    /// Serializes a type into BSON bytes but panics without handling any errors.
    ///
    /// # Panics
    ///
    /// This function panics if `bson::from_slice` returns error for:
    ///     - encountering `std::io::Error`,
    ///     - type cannot be serialized into BSON,
    ///     - invalid UTF-8 string when serializing from string,
    ///     - other deserialization errors.
    fn from_model_type(value: &T) -> Self {
        bson::to_vec(value).expect("failed to serialize a type into BSON bytes")
    }
}

fn typetest() {
    let naive = NaiveDateTime::default();
    let mut test: Vec<u8> = vec![];
    test = Vec::<u8>::from_model_type(&naive);
}