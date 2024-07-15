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

/// Duplicating macro code goes here
pub mod trx_helpers {
    use foundationdb::tuple::TuplePack;
    use super::*;

    pub fn unsaved_bool() -> bool {
        false
    }

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

    pub async fn delete_unique_index<T: TuplePack>(trx: &Transaction, index_subspace: &Subspace, index: &T, ser_key_tuple: &[u8]) -> Result<(), NonDieselDbError> {
        let key = index_subspace.pack(index);
        let Some(val) = trx.get(&key, false).await.unwrap() else {
            return Err(NonDieselDbError::TrxFail);
        };
        if val.as_ref() != ser_key_tuple {
            return Err(NonDieselDbError::IndexMismatchError);
        }
        let res = trx.clear(&key);
        Ok(res)
    }

    pub async fn delete_multi_index(trx: &Transaction, index_col_subspace: &Subspace, ser_key_tuple: &[u8]) -> Result<(), NonDieselDbError> {
        let key = index_col_subspace.pack(&ser_key_tuple);
        let Some(_empty_slice) = trx.get(&key, false).await.unwrap() else {
            return Err(NonDieselDbError::TrxFail);
        };
        let res = trx.clear(&key);
        Ok(res)
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
                    ( $( $multi_index_name ; $key ),+ ), )?
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
                use ::foundationdb::{Transaction, RangeOption};
                use ::foundationdb::tuple::Subspace;
                use $crate::db::nondiesel::{NonDieselConnection, NonDieselDbError};
                use $crate::db::nondiesel::fdb::trx_helpers::*;
                use $crate::api::EmptyResult;

                const TABLE_SUBSPACE: &[u8] = stringify!([<$table _tbl>]).as_bytes();
                const LOOK_UP_SUBSPACE: &[u8] = stringify!([<$table _lus>]).as_bytes();
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
                #[derive(PartialEq)]
                enum Col {
                    $( [<$attr_name:camel>] = $col_num ),+
                }

                fn key_subspace($($key_name: &[<$key_name:camel Type>]),+) -> Subspace {
                    Subspace::from_bytes(TABLE_SUBSPACE)
                    $(  .subspace($key_name) )+
                }

                pub struct [<$table:camel Db>] {
                    _subspace: Subspace,
                    _saved: bool,
                    _serialized_key_tuple: Vec<u8>,
                    $( $attr_name: $attr_ty ),+
                }


                impl [<$table:camel Db>] {
                    ///
                    pub fn new($( $attr_name: $attr_ty ),+) -> Self {
                        let _subspace = key_subspace($( &$key_name ),+);
                        let _serialized_key_tuple = bson::to_vec(&($( &$key_name ),+)).unwrap();
                        Self {
                            _subspace,
                            _saved: false,
                            _serialized_key_tuple,
                            $( $attr_name ),+
                        }
                    }

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
                                ).unwrap(), // TODO: Failed to deser
                            )+
                            _saved: true,
                            _serialized_key_tuple: bson::to_vec(&($( $key_name ),+)).unwrap(),
                            _subspace: key_subspace
                        };
                        Ok(Some(res))
                    }

                    pub async fn set(&self, trx: Transaction) -> Result<(), NonDieselDbError> {
                        // TODO: Remove unwrap
                        // Serialize all attributes
                        $( let $attr_name = bson::to_vec(&self.$attr_name).unwrap(); )+
                        // Primary keys
                        let key_subspace = key_subspace($( &self.$key_name ),+);
                        let ser_key_tuple = bson::to_vec(&($( &self.$key_name ),+)).unwrap();
                        $(
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

                    pub async fn delete(self, trx: Transaction) -> Result<(), NonDieselDbError> {
                        trx.clear_subspace_range(&self._subspace);
                        $(
                        $(
                        $(
                        let u_i_subspace = u_index_col_subspace(Col::[<$unique_index_name:camel>]);
                        // set_unique_index(
                        //     &trx,
                        //     u_index_col_subspace(Col::[<$unique_index_name:camel>]),
                        //     &$unique_index_name,
                        //     &key_subspace.pack(&(Col::[<$unique_index_name:camel>] as u16)),
                        //     &ser_key_tuple
                        // ).await?;
                        )+)?
                        $($(
                        delete_multi_index(
                            &trx,
                            &m_index_col_subspace(Col::[<$multi_index_name:camel>]),
                            &self._serialized_key_tuple
                        ).await?;
                        )+)?

                        )?
                        trx.commit().await.map(|_| ()).map_err(|_| NonDieselDbError::TrxCommitFail)
                    }

                    $(
                    // Same method names will ensure conflict; prohibiting declaring an attribute as
                    // both unique and multi index.
                    $($(
                    pub async fn [<get_by_ $unique_index_name>](trx: &Transaction, $unique_index_name: &[<$unique_index_name:camel Type>]) -> Result<Option<Self>, NonDieselDbError> {
                        // TODO: Remove unwraps
                        // TODO: Performance improvements by getting rid of redundant deser and ser steps
                        let index_subspace = u_index_col_subspace(Col::[<$unique_index_name:camel>]);
                        let key = index_subspace.pack(&bson::to_vec($unique_index_name).unwrap());
                        let Some(res) = trx.get(&key, false).await.unwrap() else {
                            return Ok(None);
                        };
                        let ($($ui_k_name),+) = bson::from_slice(res.as_ref()).unwrap();

                        let result = Self::get(trx, $(&$ui_k_name),+).await?;
                        Ok(result)
                    }
                    )+)?
                    $($(
                    pub async fn [<get_by_ $multi_index_name>](trx: &Transaction, $multi_index_name: &[<$multi_index_name:camel Type>]) -> Result<Vec<Self>, NonDieselDbError> {
                        // TODO: Remove unwraps
                        let index_subspace = m_index_col_subspace(Col::[<$multi_index_name:camel>]);
                        // let Some(res) = trx.get(&bson::to_vec($index_name).unwrap(), false).await.unwrap() else {
                        //     return Ok(None);
                        // };
                        todo!()
                        //Ok(Some(bson::from_slice(res.as_ref()).unwrap()))
                    }
                    )+)?
                    )?

                    pub async fn get_all(trx: &Transaction) -> Result<Vec<Self>, NonDieselDbError> {
                        // TODO: Remove unwraps
                        let index_subspace = Subspace::from_bytes(LOOK_UP_SUBSPACE);
                        let range_option = RangeOption::from(&index_subspace);
                        let res = trx.get_range(&range_option, 1, false).await.unwrap();
                        let res_iter = res.iter();
                        //while let Some(F)
                        todo!()
                        //Ok(Some(bson::from_slice(res.as_ref()).unwrap()))
                    }

                    $(
                    pub async fn [<get_ $attr_name>](&self) -> &$attr_ty {
                        &self.$attr_name
                    }

                    pub async fn [<set_ $attr_name>](&self, trx: Transaction, $attr_name: &$attr_ty) ->  Result<(), NonDieselDbError> {
                        let col = Col::[<$attr_name:camel>];
                        // Setting a primary key is prohibited
                        if PK_COLS.contains(&col) {
                            return Err(NonDieselDbError::OpProhibited);
                        }
                        let key = self._subspace.pack(&(col as u16));
                        // TODO: Remove unwrap
                        let ser = bson::to_vec($attr_name).unwrap();
                        trx.set(&key, &ser);
                        let _res = trx.commit().await.unwrap();
                        Ok(())
                    }
                    )+
                }

                pub mod column {
                    use super::*;
                    $(
                    pub mod $attr_name {
                        use super::*;

                        pub async fn get(trx: &Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>]),+) -> Result<Option<$attr_ty>, NonDieselDbError> {
                            let key = key_subspace($( [<$a_k_name _key>] ),+).pack(&(Col::[<$attr_name:camel>] as u16));
                            let Some(val_slice) = trx.get(&key, false).await.unwrap() else {
                                return Ok(None);
                            };
                            let val = bson::from_slice(val_slice.as_ref()).unwrap();
                            Ok(Some(val))
                        }

                        pub async fn set(trx: Transaction, $([<$a_k_name _key>]: &[<$a_k_name:camel Type>],)+ $attr_name: &$attr_ty) ->  Result<(), NonDieselDbError> {
                            let col = Col::[<$attr_name:camel>];
                            // Setting a primary key is prohibited
                            if PK_COLS.contains(&col) {
                                return Err(NonDieselDbError::OpProhibited);
                            }
                            let key = key_subspace($( [<$a_k_name _key>] ),+).pack(&(Col::[<$attr_name:camel>] as u16));
                            let ser = bson::to_vec($attr_name).unwrap();
                            trx.set(&key, &ser);
                            let _res = trx.commit().await.unwrap();
                            Ok(())
                        }
                    }
                    )+
                }


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
    ($name:ident -> ($key_name:ident: $key_ty:ty => $val_name:ident: $val_ty:ty)) => {};
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

