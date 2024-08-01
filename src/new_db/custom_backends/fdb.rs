use std::future::Future;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use deadpool::managed::Pool;
use foundationdb::{Database, DatabaseTransact, FdbBindingError, RetryableTransaction, Transaction, TransactError, FdbError, RangeOption};
use foundationdb::api::NetworkAutoStop;
use foundationdb::future::{FdbSlice, FdbValue};
use foundationdb::options::{DatabaseOption, TransactionOption};
use foundationdb::tuple::Subspace;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use crate::new_db::custom_backends::{DbConnection, KvKeyspace, KvTransaction, SoloManager};
use crate::new_db::error::{DbConnError, TransactionError};

#[allow(unsafe_code)]
static FDB_NETWORK_RUNNER: LazyLock<Mutex<Option<NetworkAutoStop>>> = LazyLock::new(|| {
    // Safe as long as it gets dropped on shutdown
    // See these issues:
    // https://github.com/foundationdb-rs/foundationdb-rs/issues/78
    // https://github.com/Clikengo/foundationdb-rs/issues/202
    let network = unsafe { foundationdb::boot() };
    Mutex::new(Some(network))
});

pub struct FdbConnection {
    database: Database,
    subspace: Subspace,
}

pub struct FdbConfig {
    cluster_path: String,
    base_subspace: String,
    // TransactionOption::Timeout(i32)
    max_repeat: i32,
    trx_timeout: Duration,
}

impl DbConnection for FdbConnection {
    type ConnectionPool = Pool<SoloManager<Self>>;
    type Config = FdbConfig;
    type Transaction<'db> = FdbTransaction;

    /// Starts the FoundationDB network runner thread. WIll not fail if already running, but will
    /// fail when already stopped.
    fn start() -> Result<(), DbConnError> {
        let network_runner_lock = FDB_NETWORK_RUNNER.lock()
            .map_err(|_| DbConnError::StartError("can't acquire FoundationDB network runner lock".to_string()))?;
        if network_runner_lock.is_none() {
            return Err(DbConnError::StartError("FoundationDB network runner already stopped".to_string()));
        }
        return Ok(())
    }

    /// Takes the network runner thread and drops it.
    fn stop() -> Result<(), DbConnError> {
        let mut network_runner_lock = FDB_NETWORK_RUNNER.lock()
            .map_err(|_| DbConnError::StopError("can't acquire FoundationDB network runner lock".to_string()))?;

        let Some(network_runner) = network_runner_lock.take() else {
            return Err(DbConnError::StartError("FoundationDB network runner already stopped".to_string()));
        };
        drop(network_runner);
        Ok(())
    }

    fn establish(config: &Self::Config) -> Result<Self, DbConnError> {
        let cluster_path_str = config.cluster_path.strip_prefix("fdb:").unwrap_or_else(|| &config.cluster_path);

        if Path::new(&cluster_path_str).is_file() {
            return Err(DbConnError::EstablishFail(format!("cluster file not found or unreadable: {}", cluster_path_str)))
        }

        let database = Database::from_path(cluster_path_str)?;
        database.set_option(DatabaseOption::TransactionRetryLimit(config.max_repeat))?;
        database.set_option(DatabaseOption::TransactionTimeout(config.trx_timeout.as_millis() as i32))?;
        let subspace = Subspace::from_bytes(config.base_subspace.as_bytes());

        let conn = Self {
            database,
            subspace,
        };
        Ok(conn)
    }


    async fn transact<'db, F, Fut, T>(&'db self, f: F) -> Result<T, TransactionError>
    where
        F: Fn(&Self::Transaction<'db>) -> Fut,
        Fut: Future<Output=Result<T, <Self::Transaction<'db> as KvTransaction<'db>>::ClosureError>>
    {

        let res = self.database.run(|rt_trx, maybe_committed| {
            let trx = FdbTransaction {
                trx: rt_trx,
                subspace: self.subspace.to_owned(),
            };
            f(&trx).map_err(|e| e.into())
        }).await?;
        Ok(res)
    }
}


pub struct FdbTransaction {
    trx: RetryableTransaction,
    subspace: Subspace,
}

impl FdbTransaction {
    async fn get_range_by_opt(&self, range_option: RangeOption<'_>) -> Result<Vec<FdbValue>, FdbBindingError>
    {
        let mut stream = self.trx.get_ranges_keyvalues(range_option, false);
        let mut results = Vec::new();
        while let Some(res) = stream.next().await {
            let kv = res.map_err(|e| FdbBindingError::NonRetryableFdbError(e))?;
            results.push(kv);
        }

        Ok(results)
    }
}

impl<'db> KvTransaction<'db> for FdbTransaction {
    type Keyspace = Subspace;
    type ClosureError = FdbBindingError;
    type KeyRef = [u8];
    type KeyValue = FdbValue;
    type Key = Vec<u8>;
    type ValueRef = [u8];
    type Value = FdbSlice;

    async fn get<K>(&'db self, key: K) -> Result<Option<Self::Value>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        self.trx.get(key.as_ref(), false).await.map_err(|e| e.into())
    }

    async fn get_range<K>(&self, from_key: K, to_key: K) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        let range = RangeOption::from((from_key.as_ref(), to_key.as_ref()));
        self.get_range_by_opt(range).await
    }

    async fn get_space<K>(&self, keyspace: &Self::Keyspace) -> Result<Vec<Self::KeyValue>, Self::ClosureError>
    where
        K: AsRef<Self::KeyRef>
    {
        let range = RangeOption::from(keyspace);
        self.get_range_by_opt(range).await
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

    fn clear_space(&self, keyspace: &Self::Keyspace) -> Result<(), Self::ClosureError> {
        Ok(self.trx.clear_subspace_range(keyspace))
    }
}

impl KvKeyspace for Subspace {
    type KeyRef = [u8];
    type Key = Vec<u8>;

    fn all() -> Self {
        Subspace::all()
    }

    fn from_bytes<B: Into<Self::Key>>(bytes: B) -> Self {
        Subspace::from_bytes(bytes)
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes()
    }

    fn keyspace<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self {
        self.subspace(&bytes.as_ref())
    }

    fn pack<B: AsRef<Self::KeyRef> + ?Sized>(&self, bytes: &B) -> Self::Key {
        self.pack(&bytes.as_ref())
    }

    fn unpack<B: AsRef<Self::KeyRef> + ?Sized>(&self, key: &B) -> Option<Self::Key> {
        self.unpack(&key.as_ref()).ok()
    }

    fn range(&self) -> (Self::Key, Self::Key) {
        self.range()
    }
}