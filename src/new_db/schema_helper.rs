


#[macro_export]
macro_rules! kv_table {
    (
        $table:ident $key:tt $( WITH $( UNIQUE ( $( $unique_index_name:ident ),+ ) )? $( MULTI ( $( $multi_index_name:ident ),+ ) )? INDEX )? {
            $( $attr_name:ident: $(&$attr_lt:lifetime)? $attr_ty:ty = $col_num:expr ),+
        } = $table_num:expr;

        $( $extras:item )*
    ) => {
        kv_table! {
            @private_nested {
                name: $table$(< $( $table_lt ),+ >)?,
                primary_keys: $key,
                $(
                index_parent_pk: $key,
                $(unique_indices_with_associated_pk:
                    ( $( $unique_index_name ; $key ),+ ), )?
                $(multi_indices_with_associated_pk:
                    ( $( $multi_index_name ; $key ),+ ), )?
                )?
                attributes_with_associated_pk:
                    $( ( $attr_name: $(&$attr_lt)? $attr_ty = $col_num ; $key ) ),+
                extra_items:
                    $( $extras )*
            }
        }
    };

    (
        @private_nested {
            name: $table:ident
            primary_keys: ( $( $key_name:ident ),+ ),
            $(
            index_parent_pk: ( $( $index_k_name:ident ),+ ),
            $(unique_indices_with_associated_pk:
                ( $( $unique_index_name:ident ; ( $( $ui_k_name:ident ),+ ) ),+ ), )?
            $(multi_indices_with_associated_pk:
                ( $( $multi_index_name:ident ; ( $( $mi_k_name:ident ),+ ) ),+ ), )?
            )?
            attributes_with_associated_pk:
                $( ( $attr_name:ident: $(&$attr_lt:lifetime)? $attr_ty:ty = $col_num:expr ; ( $( $a_k_name:ident ),+ ) ) ),+
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


                $( $extras )*
            }
        }
    };
}
