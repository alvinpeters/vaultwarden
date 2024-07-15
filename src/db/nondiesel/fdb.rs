use std::sync::Mutex;

use foundationdb::{Database, Transaction};
use foundationdb::api::NetworkAutoStop;
use foundationdb::tuple::Subspace;
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::db::nondiesel::{NonDieselConnection, NonDieselConnError, NonDieselDbError};

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

pub mod trx_helpers {
    use super::*;

    pub async fn set_unique_index(
        trx: &Transaction, index_subspace: Subspace, new_index: &[u8], table_key: &[u8], ser_key_tuple: &[u8]
    ) -> Result<(), NonDieselDbError> {
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

    pub async fn set_multi_index(
        trx: &Transaction, index_subspace: Subspace, new_index: &[u8], table_key: &[u8], ser_key_tuple: &[u8]
    ) -> Result<(), NonDieselDbError> {
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
}



#[macro_export]
macro_rules! fdb_table {
    (
        $table:ident $key:tt $( $( UNIQUE ( $( $unique_index_name:ident ),+ ) )? $( MULTI ( $( $multi_index_name:ident ),+ ) )? INDEX )? {
            $( $attr_name:ident: $attr_ty:ty = $col_num:expr ),+
        }

        $( $extras:item )*
    ) => {
        fdb_table! {
            @private_nested {
                name: $table,
                primary_keys: $key,
                $(
                index_parent_pk: $key,
                $(unique_indices_with_associated_pk:
                    ( $( $unique_index_name ; $key ),+ ), )?
                $(multi_indices_with_associated_pk:
                    ( $( $multi_index_name:ident ; $key ),+ ), )?
                )?
                attributes_with_associated_pk:
                    $( ( $attr_name: $attr_ty = $col_num ; $key ) ),+
                extra_items:
                    $( $extras )*
            }
        }
    };
    (
        @private_nested {
            name: $table:ident,
            primary_keys: ( $( $key_name:ident ),+ ),
            $(
            index_parent_pk: ( $( $index_k_name:ident ),+ ),
            $(unique_indices_with_associated_pk:
                ( $( $unique_index_name:ident ; ( $( $ui_k_name:ident ),+ ) ),+ ), )?
            $(multi_indices_with_associated_pk:
                ( $( $multi_index_name:ident ; ( $( $mi_k_name:ident ),+ ) ),+ ), )?
            )?
            attributes_with_associated_pk:
                $( ( $attr_name:ident: $attr_ty:ty = $col_num:expr ; ( $( $a_k_name:ident ),+ ) ) ),+
            extra_items:
                $( $extras:item )*
        }
    ) => {
        paste::paste! {
            #[allow(non_camel_case_types)]
            #[allow(non_upper_case_globals)]
            #[allow(non_snake_case)]
            #[allow(dead_code)]
            pub mod $table {
                use ::foundationdb::Transaction;
                use ::foundationdb::tuple::Subspace;
                use $crate::db::nondiesel::{NonDieselConnection, NonDieselDbError};
                use $crate::db::nondiesel::fdb::trx_helpers::*;
                use $crate::api::EmptyResult;

                const TABLE_SUBSPACE: &[u8] = stringify!([<$table _tbl>]).as_bytes();
                const PK_COLS: &[Col] = &[ $( Col::[<$key_name:camel>] ),+ ];
                $(
                $(
                const U_INDEX_SUBSPACE: &[u8] = stringify!({<$table _u_i>}).as_bytes();
                const U_INDEX_COLS: &[Col] = &[$( Col::[<$unique_index_name:camel>] ),+];

                fn u_index_col_subspace(index_col: Col) -> Subspace {
                    Subspace::from_bytes(U_INDEX_SUBSPACE).subspace(&(index_col as u16))
                }

                )?
                $(
                const M_INDEX_SUBSPACE: &[u8] = stringify!({<$table _m_i>}).as_bytes();
                const M_INDEX_COLS: &[Col] = &[$( Col::[<$multi_index_name:camel>] ),+];

                fn m_index_col_subspace(index_col: Col) -> Subspace {
                    Subspace::from_bytes(M_INDEX_SUBSPACE).subspace(&(index_col as u16))
                }
                )?
                )?

                // Needed for primary key and index types
                $( type [<$attr_name:camel Type>] = $attr_ty; )+

                #[repr(u8)]
                enum Col {
                    $( [<$attr_name:camel>] = $col_num ),+
                }

                fn key_subspace($($key_name: &[<$key_name:camel Type>]),+) -> Subspace {
                    Subspace::from_bytes(TABLE_SUBSPACE)
                    $(  .subspace($key_name) )+
                }


                #[derive(Serialize, Deserialize)]
                pub struct [<$table:camel Db>] {
                    // #[serde(skip)]
                    // _subspace: Subspace,
                    $( pub $attr_name: $attr_ty ),+
                }

                impl [<$table:camel Db>] {
                    pub async fn get(trx: &Transaction, $($key_name: &[<$key_name:camel Type>]),+) -> Result<Option<Self>, NonDieselDbError> {
                        let key_subspace = key_subspace($( $key_name ),+);
                        // TODO: Remove unwrap
                        let res = Self {
                            $( $attr_name: bson::from_slice(
                                trx.get(&key_subspace.pack(&(Col::[<$attr_name:camel>] as u16)), false)
                                    .await
                                    .unwrap() // TODO: Database error
                                    .unwrap() // TODO: Means its missing, return Ok(None)
                                    .as_ref()
                                ).unwrap() // TODO: Failed to deser
                            ),+
                        };
                        Ok(Some(res))
                    }

                    pub async fn set(&self, trx: Transaction) -> Result<(), NonDieselDbError> {
                        // TODO: Remove unwrap
                        // Serialize all attributes
                        $( let $attr_name = bson::to_vec(&self.$attr_name).unwrap(); )+
                        // Primary keys
                        let key_subspace = key_subspace($( &self.$key_name ),+);
                        $(
                        let ser_key_tuple = bson::to_vec(&($( &self.$index_k_name ),+)).unwrap();
                        $($(
                        set_unique_index(
                            &trx,
                            u_index_col_subspace(Col::[<$unique_index_name:camel>]),
                            &$unique_index_name,
                            &key_subspace.pack(&(Col::[<$unique_index_name:camel>] as u16)),
                            &ser_key_tuple
                        ).await?;
                        )+)?
                        $($(
                        set_multi_index(
                            &trx,
                            m_index_col_subspace(Col::[<$multi_index_name:camel>]),
                            &$multi_index_name,
                            &key_subspace.pack(&(Col::[<$multi_index_name:camel>] as u16)),
                            &ser_key_tuple
                        ).await?;
                        )+)?

                        )?
                        $( trx.set(&key_subspace.pack(&(Col::[<$attr_name:camel>] as u16)), &$attr_name); )+
                        trx.commit().await.map(|_| ()).map_err(|_| NonDieselDbError::TrxCommitFail)
                    }

                    pub async fn delete(trx: Transaction, $($key_name: &[<$key_name:camel Type>]),+) -> Result<(), NonDieselDbError> {
                        let key_subspace = key_subspace($( $key_name ),+);
                        trx.clear_subspace_range(&key_subspace);
                        trx.commit().await.map(|_| ()).map_err(|_| NonDieselDbError::TrxCommitFail)
                    }

                    $(
                    // Same method names will ensure conflict; prohibiting declaring an attribute as
                    // both unique and multi index.
                    $($(
                    pub async fn [<get_by_ $unique_index_name>](trx: &Transaction, $unique_index_name: &[<$unique_index_name:camel Type>]) -> Result<Option<Self>, NonDieselDbError> {
                        // TODO: Remove unwraps
                        let index_subspace = u_index_col_subspace(Col::[<$unique_index_name:camel>]);
                        // let Some(res) = trx.get(&bson::to_vec($index_name).unwrap(), false).await.unwrap() else {
                        //     return Ok(None);
                        // };
                        todo!()
                        //Ok(Some(bson::from_slice(res.as_ref()).unwrap()))
                    }
                    )+)?
                    $($(
                    pub async fn [<get_by_ $multi_index_name>](trx: &Transaction, $multi_index_name: &[<$multi_index_name:camel Type>]) -> Result<Vec<Self>, NonDieselDbError> {
                        // TODO: Remove unwraps
                        let index_subspace = set_multi_index(Col::[<$multi_index_name:camel>]);
                        // let Some(res) = trx.get(&bson::to_vec($index_name).unwrap(), false).await.unwrap() else {
                        //     return Ok(None);
                        // };
                        todo!()
                        //Ok(Some(bson::from_slice(res.as_ref()).unwrap()))
                    }
                    )+)?
                    )?
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

                impl std::borrow::Borrow<$attr_ty> for $attr_name {
                    fn borrow(&self) -> &$attr_ty {
                        &self.val
                    }
                }

                impl $attr_name {
                    fn into_inner(self) -> $attr_ty {
                        self.val
                    }

                    async fn get(trx: &Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>]),+) -> Result<Option<Self>, NonDieselDbError> {
                        let key = key_subspace($( [<$a_k_name _key>] ),+).pack(&(Col::[<$attr_name:camel>] as u16));
                        let Some(val_slice) = trx.get(&key, false).await.unwrap() else {
                            return Ok(None);
                        };
                        Ok(Some(Self {
                            val: bson::from_slice(val_slice.as_ref()).unwrap()
                        }))
                    }

                    async fn set(trx: Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>],)+ $attr_name: &$attr_ty) ->  Result<(), NonDieselDbError> {
                        let key = key_subspace($( [<$a_k_name _key>] ),+).pack(&(Col::[<$attr_name:camel>] as u16));
                        let ser = bson::to_vec($attr_name).unwrap();
                        trx.set(&key, &ser);
                        let _res = trx.commit().await.unwrap();
                        Ok(())
                    }


                }
                )+

                $( $extras )*
            }
        }
    };
}

#[macro_export]
macro_rules! fdb_key {
    ($name:ident -> $key_name:ident: $key_ty:ty) => {

    };
}

#[macro_export]
macro_rules! fdb_key_value {
    ($name:ident -> ($key_name:ident: $key_ty:ty = $val_name:ident: $val_ty:ty)) => {};
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