use std::ops::Deref;
use std::sync::{Arc, Mutex};
use foundationdb::{Database, FdbError, FdbResult, Transaction};
use foundationdb::api::NetworkAutoStop;
use crate::config::CONFIG;
use std::time::Duration;
use once_cell::sync::Lazy;
use rocket::http::tls::rustls::Connection;
use tokio::task::spawn_blocking;
use crate::db::nondiesel::{NonDieselConnection, NondieselConnectionError, NonDieselConnectionSettings, NondieselDbError, NonDieselPoolInner};
use crate::{Error, MapResult};
use crate::db::nondiesel::models::Model;
use crate::db::nondiesel::transaction::NondieselTransaction;
use crate::db::run_blocking;

static FDB_NETWORK_CLIENT: Lazy<Mutex<Option<NetworkAutoStop>>> = Lazy::new(|| {
    // Safe as long as it gets dropped on shutdown
    // See these issues:
    // https://github.com/foundationdb-rs/foundationdb-rs/issues/78
    // https://github.com/Clikengo/foundationdb-rs/issues/202
    let network = unsafe { foundationdb::boot() };
    Mutex::new(Some(network))
});

#[derive(Clone)]
pub struct FdbConnection {
    database: Arc<Database>,
}

impl NonDieselConnection for FdbConnection {
    // Starts the network runner via triggering the lazy static. Must only be run once.
    fn start() -> Result<NonDieselPoolInner<Self>, Error> {
        let Ok(mut network_opt) = FDB_NETWORK_CLIENT.lock() else {
            err!("Failed to acquire lock on the FoundationDB's network runner on start!")
        };
        if network_opt.is_none() {
            err!("FoundationDB network runner failed to start")
        }
        
        
        let db_url = CONFIG.database_url();
        let initial_conn = NonDieselPoolInner::Single(Self::establish(&db_url)?);
        Ok(initial_conn)
    }

    // Stops the network runner. Must be run only once.
    fn stop() -> Result<(), Error> {
        let Ok(mut network_opt) = FDB_NETWORK_CLIENT.lock() else {
            err!("Failed to acquire lock on the FoundationDB's network runner on stop!")
        };
        let Some(network) = network_opt.take() else {
            err!("FoundationDB network runner has already been stopped")
        };
        Ok(drop(network))
    }

    fn establish(database_url: &str) -> Result<Self, NondieselConnectionError> {
        match Database::new(Some(database_url)) {
            Ok(d) => Ok(Self {
                database: Arc::new(d),
            }),
            Err(e) => return Err(
                NondieselConnectionError::FailedToConnect("failed to connect to FoundationDB".to_string())
            )
        }
    }

    async fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<NondieselDbError>
    {
        f(self)

    }
}

pub struct FdbTransaction<M> where M: Model {
    trx_f: fn(Transaction) -> Result<M, NondieselDbError>
}

impl<M> FdbTransaction<M> where M: Model {
    fn create_trx(f: fn(Transaction) -> Result<M, NondieselDbError>) -> Self {
        Self {
            trx_f: f,
        }
    }
}

impl<M> NondieselTransaction<M> for FdbTransaction<M> where M: Model {
    type Connection = FdbConnection;

    async fn load(mut self, conn: Self::Connection) -> Result<M, NondieselDbError> {
        let trx = conn.database.create_trx().unwrap();
        // call the function
        (self.trx_f)(trx)
    }

    async fn commit(mut self, conn: Self::Connection) -> Result<(), NondieselDbError> {
        todo!()
    }
}

// Reference: https://apple.github.io/foundationdb/data-modeling.html

/// Only put the key name, which will then be used as a subspace.
/// All values are stored as bytes anyway so no point on specifying the type.
#[macro_export]
macro_rules! fdb_key_value {
    ( $name:ident ( $key_name:ident ) ) => {

    };
}

#[macro_export]
macro_rules! fdb_table {
    (
        $table:ident $( ( $( $key_name:ident ),+ ) )? {
            $( $attr:ident -> $ty:ty ),*
        }
    ) => {
        paste::paste! {
            pub mod $table {
                const SUBSPACE:&str  = stringify!($table);

                // Attribute types
                // Has to be done because of primary keys
                $( pub type [<$attr:camel>] = $ty; )*

                pub enum Column {
                    $( [<$attr:camel>] ),*
                }

                pub struct [<$table:camel Table>] {
                    $( $attr: [<$attr:camel>] ),*
                }

                impl $crate::db::nondiesel::models::Model for [<$table:camel Table>] {
                    type Transaction = $crate::db::nondiesel::fdb::FdbTransaction<Self>;
                }

                impl $crate::db::nondiesel::models::Table for [<$table:camel Table>] {
                    fn get() -> Self::Transaction {
                        todo!()
                    }

                    fn set() -> Self::Transaction {
                        todo!()
                    }
                }


            }
        }
    };
}

#[macro_export]
macro_rules! fdb_relationship {
    (
        $table_a:ident ( $key_a:ident ) <-> $table_b:ident ( $key_b:ident )
    ) => {
        paste! {

        }
    };
}