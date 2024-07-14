use std::sync::Mutex;
use foundationdb::{Database, Transaction};
use foundationdb::api::NetworkAutoStop;
use once_cell::sync::Lazy;
use crate::config::Config;
use crate::db::nondiesel::{NonDieselConnection, NonDieselConnError};

static FDB_NETWORK_CLIENT: Lazy<Mutex<Option<NetworkAutoStop>>> = Lazy::new(|| {
    // Safe as long as it gets dropped on shutdown
    // See these issues:
    // https://github.com/foundationdb-rs/foundationdb-rs/issues/78
    // https://github.com/Clikengo/foundationdb-rs/issues/202
    let network = unsafe { foundationdb::boot() };
    Mutex::new(Some(network))
});

pub(crate) struct FdbConnection {
    database: Database
}

impl NonDieselConnection for FdbConnection {
    type Config = Config;
    type Transaction = Transaction;

    // Starts the network runner via triggering the lazy static. Must only be run once.
    fn start() -> Result<(), NonDieselConnError> {
        let network_opt = FDB_NETWORK_CLIENT.lock()
            .map_err(|h| NonDieselConnError::StartFail)?;
        if network_opt.is_none() {
            return Err(NonDieselConnError::StartFail);
        }
        Ok(())
    }

    // Stops the network runner. Must be run only once.
    fn stop() -> Result<(), NonDieselConnError> {
        let Ok(mut network_opt) = FDB_NETWORK_CLIENT.lock() else {
            return Err(NonDieselConnError::StopFail)
        };
        let Some(network) = network_opt.take() else {
            return Err(NonDieselConnError::StopFail)
        };
        Ok(drop(network))
    }

    async fn establish(config: &Self::Config) -> Result<Self, NonDieselConnError> {
        let path = config.database_url();
        let database = Database::new(Some(&path)).map_err(|_| NonDieselConnError::EstablishFail)?;
        Ok(Self {
            database,
        })
    }

    fn get_trx(&self) -> Result<Self::Transaction, NonDieselConnError> {
        self.database.create_trx().map_err(|_| NonDieselConnError::NewTrxFail)
    }
}



#[macro_export]
macro_rules! fdb_table {
    // (
    //     $table:ident ( $( $key_name:ident: $key_ty:ty ),+ ) $( ( $( $index_name:ident:$index_ty:ty ),+ ) )? {
    //         $( $attr_name:ident: $attr_ty:ty = $col_num:expr ),+
    //     }
    //
    //     $( $extras:item )*
    // ) => {
    //     $table ( $( $key_name: $key_ty:ty ),+ ) $( ( $( $index_name:ident:$index_ty:ty ),+ ) )? {
    //         $( $attr_name:ident: $attr_ty:ty = $col_num:expr ),+
    //     }
    //
    //     $( $extras:item )*
    // };
    (
        $table:ident $key:tt $( <- ( $( $index_name:ident ),+ ) )? {
            $( $attr_name:ident: $attr_ty:ty = $col_num:expr ),+
        }

        $( $extras:item )*
    ) => {

        fdb_table! {
            @nested $table $key $( $key ( $( $index_name ; $key ),+ ) )? {
                $( $attr_name: $attr_ty = $col_num ; $key ),+
            }

            $( $extras )*
        }
    };
    (
        @nested $table:ident ( $( $key_name:ident ),+ ) $(  ( $( $ip_k_name:ident ),+ ) ( $( $index_name:ident ; ( $( $i_k_name:ident ),+ ) ),+ ) )? {
            $( $attr_name:ident: $attr_ty:ty = $col_num:expr ; ( $( $a_k_name:ident ),+ )  ),+
        }

        $( $extras:item )*
    ) => {
        paste::paste! {
            #[allow(non_camel_case_types)]
            #[allow(non_upper_case_globals)]
            #[allow(non_snake_case)]
            #[allow(dead_code)]
            pub mod $table {
                use ::foundationdb::Transaction;
                use ::foundationdb::tuple::Subspace;
                use ::foundationdb::tuple::TuplePack;
                use $crate::db::nondiesel::{NonDieselConnection, NonDieselDbError};
                use $crate::api::EmptyResult;

                const TABLE_SUBSPACE: &[u8] = stringify!([<$table _tbl>]).as_bytes();
                const PK_COLS: &[Column] = &[ $( Column::[<$key_name:camel>] ),+ ];
                $(
                const INDEX_SUBSPACE: &[u8] = stringify!({<$table _idx>}).as_bytes();
                const INDEX_COLS: &[Column] = &[$( Column::[<$index_name:camel>] ),+];

                fn index_subspace<T: TuplePack>(index_col: &T) -> Subspace {
                    Subspace::from_bytes(INDEX_SUBSPACE).subspace(index_col)
                }

                async fn set_index(trx: &Transaction, index_subspace: Subspace, new_index: &[u8], table_key: &[u8], ser_key_tuple: &[u8]) -> Result<(), NonDieselDbError> {
                    // TODO: Remove unwraps
                    // Check if the indexed 'column' already exists
                    if let Some(old_index) = trx.get(table_key, false).await.unwrap() {
                        let old_index_key = index_subspace.pack(&old_index.as_ref());
                        // Check if index entry exists
                        if let Some(old_index_pk) = trx.get(&old_index_key, false).await.unwrap() {
                            // Check if the old index entry matches the current primary key tuple
                            if old_index_pk.as_ref() != ser_key_tuple {
                                return Err(NonDieselDbError::IndexMismatchError);
                            }
                            if old_index.as_ref() == new_index {
                                // Old and new index matches, do nothing.
                                return Ok(());
                            }
                            // Delete it
                            trx.clear(&old_index_key);
                        } else {
                            // Indexed column already exists but not the index entry itself
                            // This might be an error but let's ignore this for now
                        }
                    }
                    let new_index_key = index_subspace.pack(&new_index);
                    if let Some(existing_pk) = trx.get(&new_index_key, false).await.unwrap() {
                        return Err(NonDieselDbError::IndexAlreadyExists);
                    }
                    // Set to current
                    trx.set(&new_index_key, ser_key_tuple);
                    // The caller should commit this
                    Ok(())
                }
                )?

                // Needed for primary key and index types
                $( type [<$attr_name:camel Type>] = $attr_ty; )+

                #[repr(u8)]
                pub enum Column {
                    $( [<$attr_name:camel>] = $col_num ),+
                }

                fn key_subspace<T: TuplePack>($($key_name: &T),+) -> Subspace {
                    Subspace::from_bytes(TABLE_SUBSPACE)
                    $(  .subspace($key_name) )+
                }


                #[derive(Serialize, Deserialize)]
                pub struct table {
                    // #[serde(skip)]
                    // _subspace: Subspace,
                    $( pub $attr_name: $attr_ty ),+
                }

                impl table {
                    pub async fn get(trx: &Transaction, $($key_name: &[<$key_name:camel Type>]),+) -> Result<Option<Self>, NonDieselDbError> {
                        let key_subspace = key_subspace($( &$key_name ),+);
                        // TODO: Remove unwrap
                        let res = Self {
                            $( $attr_name: bson::from_slice(
                                trx.get(&key_subspace.pack(&(Column::[<$attr_name:camel>] as u16)), false)
                                    .await
                                    .unwrap() // TODO: Database error
                                    .unwrap() // TODO: Means its missing, return Ok(None)
                                    .as_ref()
                                ).unwrap() // TODO: Failed to deser
                            ),+
                        };
                        Ok(Some(res))
                    }

                    pub async fn set(self, trx: Transaction, val: &table) -> Result<(), NonDieselDbError> {
                        // TODO: Remove unwrap
                        // Serialize all attributes
                        $( let $attr_name = bson::to_vec(&self.$attr_name).unwrap(); )+
                        // Primary keys
                        let key_subspace = key_subspace($( &$key_name ),+);
                        $(
                        let ser_key_tuple = bson::to_vec(&($( &self.$ip_k_name ),+)).unwrap();
                        $(
                        set_index(
                            &trx,
                            index_subspace(&(Column::[<$index_name:camel>] as u16)),
                            &$index_name,
                            &key_subspace.pack(&(Column::[<$index_name:camel>] as u16)),
                            &ser_key_tuple
                        ).await?;
                        )+
                        )?
                        $( trx.set(&key_subspace.pack(&(Column::[<$attr_name:camel>] as u16)), &$attr_name); )+
                        trx.commit().await.map(|_| ()).map_err(|_| NonDieselDbError::TrxCommitFail)
                    }
                }

                $(
                pub struct $attr_name {
                    val: $attr_ty
                }

                impl core::ops::Deref for $attr_name {
                    type Target = $attr_ty;

                    fn deref(&self) -> &Self::Target {
                        &self.val
                    }
                }

                impl $attr_name {

                    async fn get(trx: &Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>]),+) -> Result<Self, NonDieselDbError> {
                        paste::paste! {
                            // let subspace = trx.subspace(&(Column::[<$attr_name:camel>] as u16));
                        }
                        Ok(Self {
                            val: todo!()
                        })
                    }

                    async fn set(trx: Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>],)+ $attr_name: $attr_ty) ->  Result<(), NonDieselDbError> {

                        todo!()
                    }
                }
                )+

                $(
                pub mod index {
                    use super::*;

                    $(
                    pub async fn [<get_pk_by_ $index_name>](trx: &Transaction, $index_name: [<$index_name:camel Type>]) -> Result<($([<$i_k_name:camel Type>]),+), NonDieselDbError> {
                        todo!()
                    }
                    )+
                }
                )?

                $( $extras )*
            }
        }
    };
}

#[macro_export]
macro_rules! fdb_key {
    ($name:ident -> $key:ident: $ty:ty) => {

    };
}

#[macro_export]
macro_rules! fdb_key_value {
    () => {};
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