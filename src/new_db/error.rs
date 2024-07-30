use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbConnError {
    #[error("couldn't establish connection with this string: {0}")]
    EstablishFail(String)
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("failed to serialize: {0}")]
    SerializationError(String),
    #[cfg(fdb)]
    #[error("FoundationDB transaction failed: {0}")]
    FdbTrxFailed(String),
}

