use std::borrow::Cow;
use crate::new_db::error::TypeConversionError;

pub trait TryFromDbType<T>: ToOwned where T: ?Sized + ToOwned {
    fn try_from_db_type(val: &T) -> Result<Cow<'_, Self>, TypeConversionError>;
}

pub trait TryIntoDbType<T>: ToOwned where T: ?Sized + ToOwned {
    fn try_to_db_type(&self) -> Result<Cow<'_, T>, TypeConversionError>;
}

impl<T, U> TryIntoDbType<U> for T where T: ToOwned, U: TryFromDbType<T> {
    fn try_to_db_type(&self) -> Result<Cow<'_, U>, TypeConversionError> {
        U::try_from_db_type(self)
    }
}

