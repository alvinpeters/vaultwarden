use std::future::Future;
use std::sync::Arc;
use deadpool::managed::{Manager, Metrics, RecycleResult};
use crate::new_db::custom_backends::fdb::FdbTransaction;
use crate::new_db::error::{DbConnError, TransactionError};

#[cfg(fdb)]
pub mod fdb;
#[cfg(rdb)]
pub mod rdb;

pub trait DbConnection: Sized + Send + Sync {
    type Config;
    type Transaction<'db>: KvTransaction<'db>;

    /// Perform any boot-up actions. Usually not needed and will do nothing.
    fn start() -> Result<(), DbConnError> { Ok(()) }
    /// Perform any shutdown actions. Usually not needed and will do nothing.
    fn stop() -> Result<(), DbConnError> { Ok(()) }

    fn establish(config: &Self::Config) -> Result<Self, DbConnError>;

    async fn transact<'db, F, Fut, T>(&'db self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(&Self::Transaction<'db>) -> Fut,
        Fut: Future<Output = Result<T, <Self::Transaction<'db> as KvTransaction<'db>>::ClosureError>>;
}

/// Trait for key-value stores
pub trait KvTransaction<'db> {
    type Keyspace: KvKeyspace;
    type ClosureError;
    type KeyRef: ?Sized + Send;
    type KeyValue;
    type Key: AsRef<Self::ValueRef>;
    type ValueRef: ?Sized + Send;
    type Value: AsRef<Self::ValueRef>;

    async fn get<K>(&'db self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

    async fn get_range<K>(&self, from_key: K, to_key: K) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

    async fn get_space<K>(&self, keyspace: &Self::Keyspace) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

    fn set<K, V>(&self, key: K, value: V) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>,
        V: AsRef<Self::ValueRef>;

    fn clear<K>(&self, key: K) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>;

    fn clear_space(&self, keyspace: &Self::Keyspace) -> Result<(), Self::ClosureError>;

}

pub trait KvKeyspace: From<Vec<u8>> + Sized {
    // will eventually be merged
    type KeyRef: ?Sized + Send;
    type Key: AsRef<Self::KeyRef>;

    fn all() -> Self;

    fn from_bytes<B: Into<Self::Key>>(bytes: B) -> Self;

    fn as_bytes(&self) -> &[u8];

    fn keyspace<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self;

    /// Appends bytes to the prefix slice.
    fn pack<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self::Key;

    /// Just strips prefix. Returns none if prefix doesn't match.
    fn unpack<B: AsRef<Self::KeyRef> + ?Sized>(&self, key: &B) -> Option<Self::Key>;

    fn range(&self) -> (Self::Key, Self::Key);
}

pub(crate) struct SoloManager<T> where T: DbConnection {
    conn_arc: Arc<T>,
}

impl<T> SoloManager<T> where T: DbConnection {
    pub fn with_config(db_config: T::Config) -> Result<Self, DbConnError> {
        T::start()?;

        let manager = Self {
            conn_arc: Arc::new(T::establish(&db_config)?)
        };

        Ok(manager)
    }
}

impl<T> Manager for SoloManager<T> where T: DbConnection {
    type Type = Arc<T>;
    type Error = DbConnError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(self.conn_arc.clone())
    }

    async fn recycle(&self, obj: &mut Self::Type, metrics: &Metrics) -> RecycleResult<Self::Error> {
        // Just drop it, it'll be cloned lol
        Ok(())
    }
}