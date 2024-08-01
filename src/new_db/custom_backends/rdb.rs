use std::future::Future;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use deadpool::managed::Pool;
use rocksdb::{BoundColumnFamily, ColumnFamily, DB, DBCommon, DBPinnableSlice, DBWithThreadMode, ErrorKind, MultiThreaded, OptimisticTransactionDB, Transaction};
use crate::new_db::custom_backends::{DbConnection, KvKeyspace, KvTransaction, SoloManager};
use crate::new_db::error::{DbConnError, TransactionError};
use crate::new_db::SCHEMA_VERSION;

pub struct RdbConnection {
    database: OptimisticTransactionDB<MultiThreaded>,
    keyspace: RdbKeyspace,
    retry_attempts: usize,
}

pub struct RdbConfig {
    db_path: String,
    keyspace: String,
    retry_attempts: usize,
}

pub struct RdbTransaction<'db> {
    transaction: Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    keyspace: &'db RdbKeyspace,
}

impl DbConnection for RdbConnection {
    type ConnectionPool = Pool<SoloManager<Self>>;
    type Config = RdbConfig;
    type Transaction<'db> = RdbTransaction<'db>;

    fn establish(config: &Self::Config) -> Result<Self, DbConnError> {
        let database = OptimisticTransactionDB::<MultiThreaded>::open_default(&config.db_path).unwrap();

        let keyspace = RdbKeyspace::from(config.keyspace.as_str()).keyspace(SCHEMA_VERSION);

        let conn = Self {
            database,
            keyspace,
            retry_attempts: config.retry_attempts,
        };

        Ok(conn)
    }

    async fn transact<'db, F, Fut, T>(&'db self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(&Self::Transaction<'db>) -> Fut,
        Fut: Future<Output=Result<T, <Self::Transaction<'db> as KvTransaction<'db>>::ClosureError>>
    {

        let mut last_error = None;
        for i in 0..self.retry_attempts {
            let trx = RdbTransaction {
                transaction: self.database.transaction(),
                keyspace: &self.keyspace
            };
            // No need to retry read errors
            let result = f(&trx).await?;

            let Err(e) = trx.commit().await else {
                // Successful, leave
                return Ok(result);
            };
            match e.kind() {
                ErrorKind::Busy | ErrorKind::TryAgain => {
                    last_error = Some(e);
                    continue
                },
                _ => return Err(TransactionError::RdbTrxFailed(e))
            }

        }
        // Unwrapping because it's guaranteed to be Some() if the loop runs more than once
        Err(last_error.unwrap().into())
    }
}

impl<'db> RdbTransaction<'db> {
    async fn commit(self) -> Result<(), rocksdb::Error> {
        self.transaction.commit()
    }
}

impl<'db> KvTransaction<'db> for RdbTransaction<'db> {
    type Keyspace = RdbKeyspace;
    type ClosureError = TransactionError;
    type KeyRef = [u8];
    type KeyValue = (DBPinnableSlice<'db>, DBPinnableSlice<'db>);
    type Key = Vec<u8>;
    type ValueRef = [u8];
    type Value = DBPinnableSlice<'db>;

    async fn get<K>(&'db self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        // Transaction can't be Send
        tokio::task::block_in_place(|| self.transaction.get_pinned(key)).map_err(|e| e.into())
    }

    async fn get_range<K>(&self, from_key: K, to_key: K) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        todo!()
    }

    async fn get_space<K>(&self, keyspace: &Self::Keyspace) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        todo!()
    }

    fn set<K, V>(&self, key: K, value: V) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>,
        V: AsRef<Self::ValueRef>
    {
        //self.transaction.put_cf()
        todo!()
    }

    fn clear<K>(&self, key: K) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        todo!()
    }

    fn clear_space(&self, keyspace: &Self::Keyspace) -> Result<(), Self::ClosureError> {
        todo!()
    }
}

pub struct RdbKeyspace {
    bytes: Vec<u8>
}

impl<T> From<T> for RdbKeyspace where T: Into<Vec<u8>> {
    fn from(value: T) -> Self {
        Self::from_bytes(value)
    }
}

// Use Unit Separator (US) to reduce the chance of conflicting keyspace. Might still happen though.
const SEPARATOR: &[u8] = &[0x1F];

impl KvKeyspace for RdbKeyspace {
    type KeyRef = [u8];
    type Key = Vec<u8>;

    fn all() -> Self {
        Self {
            bytes: vec![]
        }
    }

    fn from_bytes<B: Into<Self::Key>>(bytes: B) -> Self {
        let mut bytes = bytes.into();
        bytes.extend_from_slice(SEPARATOR);

        Self {
            bytes
        }
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    fn keyspace<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self {
        let bytes = [self.bytes.as_slice(), bytes.as_ref()].concat();
        Self::from_bytes(bytes)
    }

    fn pack<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self::Key {
        [self.bytes.as_slice(), bytes.as_ref()].concat()
    }

    fn unpack<B: AsRef<Self::KeyRef> + ?Sized>(&self, key: &B) -> Option<Self::Key> {
        key.as_ref().strip_prefix(self.bytes.as_slice()).map(|s| s.to_vec())
    }

    fn range(&self) -> (Self::Key, Self::Key) {
        ([self.bytes.as_slice(), &[0x00]].concat(), [self.bytes.as_slice(), &[0xFF]].concat())
    }
}