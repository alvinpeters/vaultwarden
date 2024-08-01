use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::path::Path;
use deadpool::managed::Pool;
use diesel::SqliteConnection;
use diesel_async::{AsyncMysqlConnection, AsyncPgConnection};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use crate::new_db::custom_backends::{DbConnection, SoloManager};
use crate::new_db::custom_backends::fdb::FdbConnection;
use crate::new_db::error::DbConnError;

const SCHEMA_VERSION: &[u8] = b"v2";

mod models;
mod schema_helper;
mod error;
mod types;
pub mod custom_backends;

#[derive(Debug)]
pub enum DbConnType {
    FDB,
    RDB,
    MySQL,
    PGSQL,
    SQLite
}

impl Display for DbConnType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            DbConnType::FDB => "FoundationDB",
            DbConnType::RDB => "RocksDB",
            DbConnType::MySQL => "MySQL",
            DbConnType::PGSQL => "PostgreSQL",
            DbConnType::SQLite => "SQLite",
        })
    }
}

impl DbConnType {
    fn guess_type(conn_str: &str) -> Result<Self, DbConnError> {
        // Remote connection URL
        let db_type = if conn_str.starts_with("mysql:") {
            // MySQL
            let db_type = Self::MySQL;
            if cfg!(mysql) {
                db_type
            } else {
                return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
            }
        } else if conn_str.starts_with("postgresql:") || conn_str.starts_with("postgres:") {
            // PGSQL
            let db_type = Self::PGSQL;
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
                let db_type = Self::RDB;

                if cfg!(rdb) {
                    db_type
                } else {
                    return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
                }
            } else if path_extension == ".cluster" {
                let db_type = Self::FDB;
                if cfg!(fdb) {
                    db_type
                } else {
                    return Err(DbConnError::DbDisabled(db_type, conn_str.to_owned()));
                }
            } else {
                // Can't figure out the database type, let's default to SQLite for now
                let db_type = Self::SQLite;
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

pub enum DbPool {
    FDB(Pool<SoloManager<FdbConnection>>),
    RDB,
    MySQL(Pool<AsyncDieselConnectionManager<AsyncMysqlConnection>>),
    PGSQL(Pool<AsyncDieselConnectionManager<AsyncPgConnection>>),
    SQLite(Pool<AsyncDieselConnectionManager<SyncConnectionWrapper<SqliteConnection>>>)
}



// This is used to generate the main DbConn and DbPool enums, which contain one variant for each database supported
macro_rules! generate_connections {
    (
        $( @custom $custom_name:ident: $custom_ty:ty, )+
        $( @diesel $diesel_name:ident: $diesel_ty:ty ),+
    ) => {
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Eq, PartialEq)]
        pub enum DbConnType {
            $( $custom_name, )+
            $( $diesel_name, )+
        }

        #[derive(Debug)]
        pub struct DbConnOptions {
            pub init_stmts: String,
        }

        $( // Based on <https://stackoverflow.com/a/57717533>.
        #[cfg($diesel_name)]
        impl CustomizeConnection<$diesel_ty, diesel::r2d2::Error> for DbConnOptions {
            fn on_acquire(&self, conn: &mut $diesel_ty) -> Result<(), diesel::r2d2::Error> {
                if !self.init_stmts.is_empty() {
                    conn.batch_execute(&self.init_stmts).map_err(diesel::r2d2::Error::QueryError)?;
                }
                Ok(())
            }
        })+

    };
}