use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::error::Error as ErrorTrait;
use std::future::Future;
use chrono::NaiveDateTime;
use deadpool::managed::{Manager, Metrics, RecycleResult};
use diesel::expression::is_aggregate::No;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::Error;


pub mod fdb;
pub mod types;

pub(crate) trait NonDieselConfig: Send + Sync {
    fn new_config(config: &Config) -> Self;
}

pub(crate) trait NonDieselConnection: Sized + Send {
    type Config: NonDieselConfig;
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
        C::start()?;
        Ok(Self {
            db_config: C::Config::new_config(config)
        })
    }
}

impl<C> Drop for NonDieselConnManager<C> where C: NonDieselConnection {
    fn drop(&mut self) {
        C::stop().expect("failed to stop database connector")
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

// Perhaps we should start using thiserror

pub(crate) enum TrxError {
    TrxCommitFail,
    DeserializeError,
    SerializeError,
}

#[derive(Debug)]
pub(crate) enum NonDieselDbError {
    TrxFail,
    TrxCommitFail,
    IndexAlreadyExists,
    SerializationFail,
    PkAlreadyExists,
    OpProhibited,
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


pub(crate) trait TryFromModelType<T>: Sized {
    fn try_from_model_type(value: T) -> Self;
}

pub(crate) trait TryIntoModelType<T>: Sized {
    fn try_into_model_type(self) -> T;
}
