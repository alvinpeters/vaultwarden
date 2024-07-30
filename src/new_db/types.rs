use std::borrow::Cow;
use std::ops::Deref;

pub trait DbType<T> {
    fn to_db_type(&self) -> Cow<'_, T>;

    fn from_db_type(val: &T) -> Cow<'_, T>;
}


pub trait AsSlice {
    fn as_slice(&self) -> Cow<'_, [u8]>;
}

pub trait TryFromSlice: Sized {
    fn try_from_slice(val: impl AsRef<[u8]>) -> Self;
}

/// Avoids unnecessary cloning of string bytes
impl AsSlice for String {
    fn as_slice(&self) -> Cow<'_, [u8]> {
        Cow::from(self.as_bytes())
    }
}

impl AsSlice for str {
    fn as_slice(&self) -> Cow<'_, [u8]> {
        Cow::from(self.as_bytes())
    }
}

impl AsSlice for u64 {
    fn as_slice(&self) -> Cow<'_, [u8]> {
        Cow::from(self.to_le_bytes().to_vec())
    }
}

fn test() {
    let str = "Bwqbq";
    let vec = vec![0u8; 50];
    let num = str.as_slice();
    let testt = Cow::from(vec);
    let test = Cow::from(str.as_bytes());
}