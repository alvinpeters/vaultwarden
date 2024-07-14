use std::fmt::{Debug, Display, Formatter};
use std::error::Error as ErrorTrait;
use std::future::Future;
use deadpool::managed::{Manager, Metrics, RecycleResult};
use diesel::expression::is_aggregate::No;
use crate::config::Config;
use crate::Error;


pub mod fdb;

pub(crate) trait NonDieselConnection: Sized + Send {
    type Config: Send + Sync;
    type Transaction;

    fn start() -> Result<(), NonDieselConnError>;

    fn stop() -> Result<(), NonDieselConnError>;

    fn establish(config: &Self::Config) -> impl std::future::Future<Output = Result<Self, NonDieselConnError>> + std::marker::Send;

    fn get_trx(&self) -> Result<Self::Transaction, NonDieselConnError>;
}

pub(crate) struct NonDieselConnManager<C> where C: NonDieselConnection {
    db_config: C::Config
}

impl<C> NonDieselConnManager<C> where C: NonDieselConnection {
    pub(crate) fn new(config: &Config) -> Result<Self, Error> {
        todo!()
    }
}

impl<C> Manager for NonDieselConnManager<C> where C: NonDieselConnection {
    type Type = C;
    type Error = NonDieselConnError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        C::establish(&self.db_config).await
    }

    async fn recycle(&self, obj: &mut Self::Type, metrics: &Metrics) -> RecycleResult<Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum NonDieselDbError {
    TrxFail,
    TrxCommitFail,
    IndexAlreadyExists,
    PkAlreadyExists,
    IndexMismatchError
}

impl Display for NonDieselDbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl ErrorTrait for NonDieselDbError {}

#[derive(Debug)]
pub(crate) enum NonDieselConnError {
    CreatePoolFail,
    GetConnFail,
    StartFail,
    StopFail,
    EstablishFail,
    NewTrxFail,
}

impl Display for NonDieselConnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl ErrorTrait for NonDieselConnError {}