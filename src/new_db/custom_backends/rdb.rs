use std::future::Future;
use std::path::PathBuf;
use rocksdb::{DB, DBCommon, DBPinnableSlice, DBWithThreadMode, MultiThreaded, OptimisticTransactionDB, Transaction};
use crate::new_db::custom_backends::{DbConnection, KvKeyspace, KvTransaction};
use crate::new_db::error::{DbConnError, TransactionError};

pub struct RdbConnection {

}

pub struct RdbConfig {
    db_path: String,
    main_keyspace: String,
}

pub struct RdbTransaction {
    //transaction: Transaction<'db, DB>
}

impl DbConnection for RdbConnection {
    type Config = RdbConfig;
    type Transaction = RdbTransaction;

    fn establish(config: &Self::Config) -> Result<Self, DbConnError> {
        let db = OptimisticTransactionDB::<MultiThreaded>::open_default(&config.db_path).unwrap();
        todo!()
    }

    async fn transact<F, Fut, TrxErr, T>(&self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(Self::Transaction) -> Fut,
        Fut: Future<Output=Result<T, TrxErr>>,
        TrxErr: Into<<Self::Transaction as KvTransaction>::ClosureError>
    {
        todo!()
    }
}

impl KvTransaction for RdbTransaction {
    type Keyspace = RdbKeyspace;
    type ClosureError = ();
    type KeyRef = [u8];
    type KeyValue = ();
    type Key = Vec<u8>;
    type ValueRef = [u8];
    type Value = Vec<u8>;

    async fn get<'a, K>(&'a self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        todo!()
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

    fn keyspace<B: AsRef<Self::KeyRef>>(&self, bytes: &B) -> Self {
        let bytes = [self.bytes.as_slice(), bytes.as_ref()].concat();
        Self::from_bytes(bytes)
    }

    fn pack<B: AsRef<Self::KeyRef>>(&self, bytes: &B) -> Self::Key {
        [self.bytes.as_slice(), bytes.as_ref()].concat()
    }

    fn unpack<B: AsRef<Self::KeyRef>>(&self, key: &B) -> Option<Self::Key> {
        key.as_ref().strip_prefix(self.bytes.as_slice()).map(|s| s.to_vec())
    }

    fn range(&self) -> (Self::Key, Self::Key) {
        ([self.bytes.as_slice(), &[0x00]].concat(), [self.bytes.as_slice(), &[0xFF]].concat())
    }
}