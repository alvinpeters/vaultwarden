// use std::path::Path;
// use std::sync::Arc;
// use foundationdb::{Database, FdbError, FdbResult, Transaction};
// use foundationdb::api::NetworkAutoStop;
// use diesel::r2d2::{ManageConnection, Pool};
// use crate::config::CONFIG;
// use std::time::Duration;
// use crate::MapResult;
// 
// pub struct FdbConnection {
//     database: Arc<Database>,
// }
// 
// impl FdbConnection {
//     pub fn create_trx(&self) -> FdbResult<Transaction> {
//         self.database.create_trx()
//     }
// }
// 
// pub struct FdbClient {
//     network: Option<NetworkAutoStop>,
//     database: Arc<Database>,
//     retryable: bool
// }
// 
// impl FdbClient {
//     pub fn start(cluster_file: AsRef<Path>) -> Result<Self, FdbError> {
//         let network = unsafe { foundationdb::boot() };
//         let database = Database::new(cluster_file)?;
//         Self {
//             network: Some(network),
//             database: Arc::new(database),
//             retryable: false
//         }
//     }
// }
// 
// impl Drop for FdbClient {
//     fn drop(&mut self) {
//         let network = self.network.take().unwrap();
//         drop(network);
//     }
// }
// 
// impl ManageConnection for FdbClient {
//     type Connection = FdbConnection;
//     type Error = FdbError;
// 
//     fn connect(&self) -> Result<Self::Connection, Self::Error> {
//         FdbConnection::connect(None)
//     }
// 
//     fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
//         // Always valid for now
//         Ok(())
//     }
// 
//     fn has_broken(&self, conn: &mut Self::Connection) -> bool {
//         // Never broken for now
//         false
//     }
// }
// 
// pub(super) fn get_connection_manager(path: &str) -> FdbClient {
//     
// }
// 
// pub(super) fn build_pool(fdb_client: FdbClient) -> Result<Pool<FdbClient>> {
//     Pool::builder()
//         .max_size(CONFIG.database_max_conns())
//         .connection_timeout(Duration::from_secs(CONFIG.database_timeout()))
//         .build(fdb_client)
//         .map_res("Failed to create pool")?;
// }
// 
// #[cfg(test)]
// mod tests {
//     use super::*;
// 
//     #[test]
//     fn connect_to_db() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
