use crate::db::nondiesel::models::Model;
use crate::db::nondiesel::{NonDieselConnection, NondieselDbError};

pub trait NondieselTransaction<M>: Sized where {
    type Connection: NonDieselConnection;

    async fn load(self, conn: Self::Connection) -> Result<M, NondieselDbError>;

    async fn commit(self, conn: Self::Connection) -> Result<(), NondieselDbError>;
}