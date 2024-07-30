use std::future::Future;
use crate::new_db::error::{DbConnError, TransactionError};

#[cfg(fdb)]
pub mod fdb;

pub trait DbConnection: Sized {
    type Transaction: DbTransaction;

    /// Perform any boot-up actions. Usually not needed and will do nothing.
    fn start() -> Result<(), DbConnError> { Ok(()) }
    /// Perform any shutdown actions. Usually not needed and will do nothing.
    fn stop() -> Result<(), DbConnError> { Ok(()) }

    fn establish(connection_str: &str) -> Result<Self, DbConnError>;

    async fn transact<F, Fut, TrxErr, T>(&self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(Self::Transaction) -> Fut,
        Fut: Future<Output = Result<T, TrxErr>>,
        TrxErr: Into<<Self::Transaction as DbTransaction>::ClosureError>;
}

pub trait DbTransaction {
    type Keyspace;
    type ClosureError;
    type KeyRef: ?Sized;
    type Key: AsRef<Self::ValueRef>;
    type ValueRef: ?Sized;
    type Value: AsRef<Self::ValueRef>;



    async fn get<K>(&self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

    fn set<K, V>(&self, key: K, value: V) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>,
        V: AsRef<Self::ValueRef>;

    fn clear<K>(&self, key: K) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

}