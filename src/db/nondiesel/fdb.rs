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
        $table:ident $key:tt $( ( $( $index_name:ident: $index_ty:ty ),+ ) )? {
            $( $attr_name:ident: $attr_ty:ty = $col_num:expr ),+
        }

        $( $extras:item )*
    ) => {

        fdb_table! {
            @nested $table $key $( ( $( $index_name: $index_ty ; $key ),+ ) )? {
                $( $attr_name: $attr_ty = $col_num ; $key ),+
            }

            $( $extras )*
        }
    };
    (
        @nested $table:ident ( $( $key_name:ident: $key_ty:ty ),+ ) $( ( $( $index_name:ident:$index_ty:ty ; ( $( $i_k_name:ident: $i_k_ty:ty ),+ ) ),+ ) )? {
            $( $attr_name:ident: $attr_ty:ty = $col_num:expr ; ( $( $a_k_name:ident: $a_k_ty:ty ),+ )  ),+
        }

        $( $extras:item )*
    ) => {
        // Create sub-macro to handle method generation

        #[allow(non_camel_case_types)]
        #[allow(non_upper_case_globals)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        pub mod $table {
            use ::foundationdb::Transaction;
            use ::foundationdb::tuple::Subspace;
            use foundationdb::tuple::TuplePack;
            use $crate::db::nondiesel::{NonDieselConnection, NonDieselDbError};
            use $crate::api::EmptyResult;

            paste::paste! {
                const SUBSPACE: &[u8] = stringify!($table).as_bytes();
                const PK_COLS: &[Column] = &[ $( Column::[<$key_name:camel>] ),+ ];
                $(
                const INDEX_SUBSPACE: &[u8] = stringify!({<$table _INDEX>}).as_bytes();
                const INDEX_COLS: &[Column] = &[$( Column::[<$index_name:camel>] ),+];
                )?

                #[repr(u8)]
                pub enum Column {
                    $( [<$attr_name:camel>] = $col_num ),+
                }
            }

            fn key_subspace<T: TuplePack>($($key_name: &T),+) -> Subspace {
                Subspace::from_bytes(SUBSPACE)
                $(  .subspace($key_name) )+
            }


            #[derive(Serialize, Deserialize)]
            pub struct table {
                // #[serde(skip)]
                // _subspace: Subspace,
                $( pub $attr_name: $attr_ty ),+
            }

            impl table {
                pub async fn get(trx: &Transaction, $($key_name: &$key_ty),+) -> Result<Self, NonDieselDbError> {
                    let key_subspace = key_subspace($( &$key_name ),+);
                    // TODO: Remove unwrap

                    todo!()
                }

                pub async fn set(self, trx: Transaction, val: &table) -> EmptyResult {
                    // TODO: Remove unwrap
                    $( let $attr_name = bson::to_vec(&self.$attr_name).unwrap(); )+
                    // Primary keys
                    let key_subspace = key_subspace($( &$key_name ),+);

                    paste::paste! {
                        $( trx.set(&key_subspace.pack(&(Column::[<$attr_name:camel>] as u16)), &$attr_name); )+
                    }
                    trx.commit().await;
                    todo!()
                }

                async fn update_index_if_ne(trx: &Transaction, subspace: Subspace, old_index: &[u8], new_index: &[u8], pk: &[u8]) {
                    if old_index == new_index {
                        return;
                    }
                    trx.clear(&subspace.subspace(&pk).pack(&old_index));
                    trx.set(&subspace.subspace(&pk).pack(&new_index), pk);
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

                async fn get(trx: &Transaction, $(paste::paste!([<$a_k_name _key>]): &$a_k_ty),+) -> Result<Self, NonDieselDbError> {
                    paste::paste! {
                        // let subspace = trx.subspace(&(Column::[<$attr_name:camel>] as u16));
                    }
                    todo!()
                }

                async fn set(trx: Transaction, $(paste::paste!([<$a_k_name _key>]): &$a_k_ty,)+ $attr_name: $attr_ty) ->  Result<(), NonDieselDbError> {

                    todo!()
                }
            }
            )+

            $(
            pub mod indices {
                use super::*;

                $(
                pub async fn $index_name (trx: &Transaction, $index_name: $index_ty) -> Result<($($i_k_ty),+), NonDieselDbError> {

                    todo!()
                }
                )+
            }
            )?

            $( $extras )*
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