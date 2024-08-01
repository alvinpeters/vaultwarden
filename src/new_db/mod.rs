use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::time::Duration;
use deadpool::managed::{Object, Pool, Timeouts};
use deadpool::Runtime;
use deadpool::managed::PoolError;
#[cfg(feature = "new_db_diesel")]
use diesel::SqliteConnection;
use diesel_async::{AsyncMysqlConnection, AsyncPgConnection};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;

use crate::new_db::custom_backends::{DbConnection, SoloManager};
use crate::new_db::custom_backends::fdb::FdbConnection;
use crate::new_db::custom_backends::rdb::RdbConnection;
use crate::new_db::error::DbConnError;

const SCHEMA_VERSION: &[u8] = b"v2";

pub mod models;
pub mod schema_helper;
pub mod error;
mod types;
pub mod custom_backends;



// This is used to generate the main DbConn and DbPool enums, which contain one variant for each database supported
macro_rules! generate_connections {
    (
        $( @custom $custom_name:ident: $custom_ty:ty = $custom_db_string_name:expr, )+
        $( @diesel $diesel_name:ident: $diesel_ty:ty = $diesel_db_string_name:expr ),+
    ) => {
        #[allow(dead_code, non_camel_case_types)]
        #[derive(Eq, PartialEq, Debug)]
        pub enum DbConnType {
            $( $custom_name, )+
            $( $diesel_name, )+
        }

        impl Display for DbConnType {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", match self {
                    $( DbConnType::$custom_name => $custom_db_string_name, )+
                    $( DbConnType::$diesel_name => $diesel_db_string_name, )+
                })
            }
        }



        #[allow(dead_code, non_camel_case_types)]
        pub enum DbPool {
            $( $custom_name (Pool<<$custom_ty as DbConnection>::PoolManager>), )+
            $( $diesel_name (Pool<AsyncDieselConnectionManager< $diesel_ty >>), )+
        }

        #[allow(dead_code, non_camel_case_types)]
        pub enum DbConn {
            $( $custom_name ( Object<<$custom_ty as DbConnection>::PoolManager> ), )+
            $( $diesel_name ( Object<AsyncDieselConnectionManager< $diesel_ty >> ), )+
        }

        impl DbPool {
            pub fn from_config(config: &crate::config::Config) -> Result<Self, DbConnError> {
                let url= config.database_url();
                let db_type = DbConnType::guess_type(&url)?;

                match db_type {
                    $( DbConnType::$custom_name => {
                        #[cfg($custom_name)] {
                            let manager = <$custom_ty as DbConnection>::PoolManager::with_config(&config)?;
                            let pool = Pool::builder(manager);

                            Ok(DbPool::$custom_name(pool.build()?))
                        }

                        #[cfg(not($custom_name))]
                        unreachable!("Database backend not enabled")
                    }, )+
                    $( DbConnType::$diesel_name => {
                        #[cfg($diesel_name)] {
                            let manager = AsyncDieselConnectionManager::new(url);

                            let mut timeouts = Timeouts::new();
                            timeouts.create = Some(Duration::from_secs(config.database_timeout()));
                            timeouts.wait = Some(Duration::from_secs(config.database_timeout()));
                            timeouts.recycle = Some(Duration::from_secs(config.database_timeout()));

                            let pool = Pool::builder(manager)
                                .max_size(config.database_max_conns() as usize)
                                .timeouts(timeouts)
                                .runtime(Runtime::Tokio1);

                            Ok(DbPool::$diesel_name(pool.build()?))
                        }

                        #[cfg(not($diesel_name))]
                        unreachable!("Database backend not enabled")
                    } ),+
                }
            }

            pub async fn get(&self) -> Result<DbConn, PoolError<DbConnError>> {
                match self {
                    $( DbPool::$custom_name (pool) => {
                        let conn = pool.get().await?;
                        Ok(DbConn::$custom_name (conn))
                    }, )+
                    $( DbPool::$diesel_name (pool) => {
                        let conn = pool.get().await.map_err(|e| PoolError::Backend(DbConnError::from(e)))?;
                        Ok(DbConn::$diesel_name (conn))
                    } ),+
                }
            }

        }
    };
}

generate_connections! {
    @custom fdb: FdbConnection = "FoundationDB",
    @custom rdb: RdbConnection = "RocksDB",
    @diesel mysql: AsyncMysqlConnection = "MySQL",
    @diesel pgsql: AsyncPgConnection = "PostgreSQL",
    @diesel sqlite: SyncConnectionWrapper<SqliteConnection> = "SQLite"
}

impl DbConnType {
    fn guess_type(conn_str: &str) -> Result<Self, DbConnError> {
        // Remote connection URL
        let db_type = if conn_str.starts_with("mysql:") {
            // MySQL
            let db_type = Self::mysql;
            if cfg!(mysql) {
                db_type
            } else {
                return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
            }
        } else if conn_str.starts_with("postgresql:") || conn_str.starts_with("postgres:") {
            // PGSQL
            let db_type = Self::pgsql;
            if cfg!(postgresql) {
                db_type
            } else {
                return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
            }
        } else {
            // String refers to a file/directory
            // Make a Path type, but don't check whether it actually exist, only if it's a directory.
            let db_path = Path::new(conn_str);
            let path_extension = db_path.extension().unwrap_or_else(|| OsStr::new(""));

            if db_path.is_dir() {
                let db_type = Self::rdb;

                if cfg!(rdb) {
                    db_type
                } else {
                    return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
                }
            } else if path_extension == ".cluster" {
                let db_type = Self::fdb;
                if cfg!(fdb) {
                    db_type
                } else {
                    return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
                }
            } else {
                // Can't figure out the database type, let's default to SQLite for now
                let db_type = Self::sqlite;
                if cfg!(sqlite) {
                    db_type
                } else {
                    return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
                }
            }
        };

        Ok(db_type)
    }
}