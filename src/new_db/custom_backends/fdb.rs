use std::future::Future;
use std::mem;
use foundationdb::{Database, FdbBindingError, RetryableTransaction};
use foundationdb::future::FdbSlice;
use foundationdb::tuple::Subspace;
use crate::new_db::custom_backends::{DbConnection, DbTransaction};
use crate::new_db::error::{DbConnError, TransactionError};

pub struct FdbConnection {
    database: Database,
    subspace: Subspace,
}

#[allow(unsafe_code)]
fn start() {
    unsafe {
        let conn: FdbConnection = mem::zeroed();
    }
}

impl DbConnection for FdbConnection {
    type Transaction = FdbTransaction;

    fn establish() -> Result<Self, DbConnError> {
        todo!()
    }

    async fn transact<F, Fut, TrxErr, T>(&self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(Self::Transaction) -> Fut,
        Fut: Future<Output=Result<T, TrxErr>>,
        TrxErr: Into<<Self::Transaction as DbTransaction>::ClosureError>
    {
        self.database.run(|rt_trx, maybe_committed| {
            let trx = FdbTransaction {
                trx: rt_trx,
                subspace: self.subspace.to_owned(),
            };
            f(trx)
        })
    }
}

pub struct FdbTransaction {
    trx: RetryableTransaction,
    subspace: Subspace,
}

impl DbTransaction for FdbTransaction {
    type Keyspace = Subspace;
    type ClosureError = FdbBindingError;
    type KeyRef = [u8];
    type Key = Vec<u8>;
    type ValueRef = [u8];
    type Value = FdbSlice;

    async fn get<K>(&self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        self.trx.get(key.as_ref(), false).await.map_err(|e| e.into())
    }

    fn set<K, V>(&self, key: K, value: V) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>,
        V: AsRef<Self::ValueRef>
    {
        Ok(self.trx.set(key.as_ref(), value.as_ref()))
    }

    fn clear<K>(&self, key: K) -> Result<(), Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        Ok(self.trx.clear(key.as_ref()))
    }
}

impl From<FdbBindingError> for TransactionError {
    fn from(value: FdbBindingError) -> Self {
        todo!()
    }
}