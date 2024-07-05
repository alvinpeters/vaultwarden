//use diesel::r2d2::{ManageConnection, Pool};

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use crate::CONFIG;

#[cfg(fdb)]
pub mod fdb;

pub mod transaction;
pub mod models;

#[derive(Clone)]
enum NonDieselPoolInner<C> where C: NonDieselConnection {
    Single(C),
    Multi(Arc<Mutex<Vec<C>>>), // An actual pool, unimplemented
}

/// A "pool" trait to act as a surrogate for non-diesel based database
/// implementations like FoundationDB.
#[derive(Clone)]
pub(super) struct NonDieselPool<C> where C: NonDieselConnection {
    inner: NonDieselPoolInner<C>,
    connection_timeout: Duration,
    max_size: usize,
}

impl<C> NonDieselPool<C> where C: NonDieselConnection {
    pub(super) fn new() -> Result<Self, crate::Error> {
        let init_conn = C::start()?;
        let max_size = match &init_conn {
            NonDieselPoolInner::Single(_) => Semaphore::MAX_PERMITS,
            NonDieselPoolInner::Multi(_) => CONFIG.database_max_conns() as usize
        };
        let timeout_duration = CONFIG.database_timeout();
        Ok(Self {
            inner: init_conn,
            connection_timeout: Duration::from_secs(timeout_duration),
            max_size,
        })
    }

    pub(super) async fn get(&self) -> Result<C, NondieselConnectionError> {
        match &self.inner {
            NonDieselPoolInner::Single(conn) => {
                Ok(conn.clone())
            },
            NonDieselPoolInner::Multi(_) => { unimplemented!() }
        }
    }

    pub(super) async fn get_timeout(&self, duration: Duration) -> Result<C, NondieselConnectionError> {
        let timeout = if duration < self.connection_timeout {
            duration
        } else {
            self.connection_timeout
        };
        let Ok(res) = tokio::time::timeout(timeout, self.get()).await else {
            return Err(NondieselConnectionError::FailedToConnect("timed out".to_string()))
        };
        match res {
            Ok(c) => Ok(c),
            Err(e) => Err(e)
        }
    }

    pub fn max_size(&self) -> usize {
        match &self.inner {
            NonDieselPoolInner::Single(_) => {
                // Since it's just cloned repeatedly, there's no real max 'connection'
                Semaphore::MAX_PERMITS
            },
            NonDieselPoolInner::Multi(_) => { unimplemented!() }
        }
    }
}

#[derive(Copy, Clone)]
struct NonDieselConnectionSettings {
    timeout: Duration,
}

/// Based on diesel::connection::Connection but simplified whenever possible
pub(super) trait NonDieselConnection: Sized + Clone {
    /// Method to start up a database's connector. Should be called once and cannot be
    /// called after the pool has been stopped.
    /// Should also return a connection wrapped in NonDieselPoolInner to let the pool know
    /// the kind of pool the database supports.
    fn start() -> Result<NonDieselPoolInner<Self>, crate::Error>;

    fn stop() -> Result<(), crate::Error>;

    fn establish(database_url: &str) -> Result<Self, NondieselConnectionError>;

    async fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where F: FnOnce(&mut Self) -> Result<T, E>,
          E: From<NondieselDbError>;
}

#[derive(Debug)]
pub enum NondieselConnectionError {
    BadConnection(String),
    FailedToConnect(String),
    InvalidConnectionUrl(String),
    CouldntSetupConfiguration(NondieselDbError),
}

impl Display for NondieselConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NondieselConnectionError::BadConnection(s) => write!(f, "Bad connection: {}", s),
            NondieselConnectionError::FailedToConnect(s) => write!(f, "Failed to connect with the database: {}", s),
            NondieselConnectionError::InvalidConnectionUrl(s) => write!(f, "Invalid connection URL: {}", s),
            NondieselConnectionError::CouldntSetupConfiguration(e) => std::fmt::Display::fmt(&e, f),
        }
    }
}

impl Error for NondieselConnectionError {

}

/// Query errors
#[derive(Debug)]
pub enum NondieselDbError {
    NotFound(String), // key/s
}

impl Display for NondieselDbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NondieselDbError::NotFound(s) => write!(f, "Key not found: {}", s)
        }
    }
}

impl Error for NondieselDbError {}