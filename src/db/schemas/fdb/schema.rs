use crate::db::__fdb_schema::group::table;
use crate::{fdb_table, fdb_key};

fdb_table! {
    attachments (id) {
        id: String = 0,
        cipher_uuid: String = 1,
        file_name: String = 2,
        file_size: u64 = 3,
        akey: Option<String> = 4
    }
}

fdb_table! {
    cipher (uuid) {
        uuid: String = 0,
        created_at: bson::DateTime = 1,
        updated_at: bson::DateTime = 2,
        user_uuid: Option<String> = 3,
        organization_uuid: String = 4,
        key: Option<String> = 5,
        atype: i32 = 6,
        name: String = 7,
        notes: Option<String> = 8,
        fields: Option<String> = 9,
        data: String = 10,
        password_history: Option<String> = 11,
        deleted_at: Option<bson::DateTime> = 12,
        reprompt: Option<i32> = 13
    }
}

// TODO: cipher (uuid) <-> collection (uuid)

fdb_table! {
    collection (uuid) {
        uuid: String = 0,
        org_uuid: String = 1,
        name: String = 2,
        external_id: Option<String> = 3
    }
}

fdb_table! {
    device (uuid, user_uuid) {
        uuid: String = 0,
        user_uuid: String = 1,
        created_at: bson::DateTime = 2,
        updated_at: bson::DateTime = 3,
        name: String = 4,
        atype: i32 = 5,
        push_uuid: Option<String> = 6,
        push_token: Option<String> = 7,
        refresh_token: String = 8,
        twofactor_remember: String = 9
    }
}

fdb_table! {
    event (uuid) {
        uuid: String = 0,
        event_type: i32 = 1,
        user_uuid: Option<String> = 2,
        org_uuid: Option<String> = 3,
        cipher_uuid: Option<String> = 4,
        collection_uuid: Option<String> = 5,
        group_uuid: Option<String> = 6,
        org_user_uuid: Option<String> = 7,
        act_user_uuid: Option<String> = 8,
        device_type: Option<i32> = 9,
        ip_address: Option<String> = 10,
        event_date: bson::DateTime = 11,
        policy_uuid: Option<String> = 12,
        provider_uuid: Option<String> = 13,
        provider_user_uuid: Option<String> = 14,
        provider_org_uuid: Option<String> = 16
    }
}

// TODO: favorites: user (uuid) -> cipher (uuid)

fdb_table! {
    folder (uuid) {
        uuid: String = 0,
        created_at: bson::DateTime = 1,
        updated_at: bson::DateTime = 2,
        user_uuid: String = 3,
        name: String = 4
    }
}

// TODO: cipher (uuid) <-> folder (uuid)

fdb_key!(invitations -> email: String);

fdb_table! {
    org_policy (uuid) {
        uuid: String = 0,
        org_uuid: String = 1,
        atype: i32 = 2,
        enable: bool = 3,
        data: String = 4
    }
}

fdb_table! {
    organization (uuid) {
        uuid: String = 0,
        name: String = 1,
        billing_email: String = 2,
        private_key: Option<String> = 3,
        public_key: String = 4
    }
}

fdb_table! {
    send (uuid) {
        uuid: String = 0,
        user_uuid: Option<String> = 1,
        organization_uuid: Option<String> = 2,
        name: String = 3,
        notes: Option<String> = 4,
        atype: i32 = 5,
        data: String = 6,
        akey: String = 7,
        password_hash: Option<bson::Binary> = 8,
        password_salt: Option<bson::Binary> = 9,
        password_iter: Option<i32> = 10,
        max_access_count: Option<i32> = 11,
        access_count: i32 = 12,
        creation_date: bson::DateTime = 13,
        revision_date: bson::DateTime = 14,
        expiration_date: Option<bson::DateTime> = 15,
        deletion_Date: bson::DateTime = 16,
        disabled: bool = 17,
        hide_email: Option<bool> = 18
    }
}

fdb_table! {
    twofactor (uuid) {
        uuid: String = 0,
        user_uuid: String = 1,
        atype: i32 = 2,
        enabled: bool = 3,
        data: String = 4,
        last_used: i64 = 5
    }
}

fdb_table! {
    twofactor_incomplete (user_uuid, device_uuid) {
        user_uuid: String = 0,
        device_uuid: String = 1,
        device_name: String = 2,
        login_time: bson::DateTime = 3,
        ip_address: String = 4
    }
}

fdb_table! {
    user (uuid) {
        uuid: String = 0,
        enabled: bool = 1,
        created_at: bson::DateTime = 2,
        updated_at: bson::DateTime = 3,
        verified_at: Option<bson::DateTime> = 4,
        last_verifying_at: Option<bson::DateTime> = 5,
        login_verify_count: i32 = 6,
        email: String = 7,
        email_new: Option<String> = 8,
        email_new_token: Option<String> = 9,
        name: String = 10,
        password_hash: bson::Binary = 11,
        salt: bson::Binary = 12,
        password_iterations: i32 = 13,
        password_hint: Option<String> = 14,
        akey: String = 15,
        private_key: Option<String> = 16,
        public_key: Option<String> = 17,
        totp_secret: Option<String> = 18,
        totp_recover: Option<String> = 19,
        security_stamp: String = 20,
        stamp_exception: Option<String> = 21,
        equivalent_domains: String = 22,
        excluded_globals: String = 23,
        client_kdf_type: i32 = 24,
        client_kdf_iter: i32 = 25,
        client_kdf_memory: Option<i32> = 26,
        client_kdf_parallelism: Option<i32> = 27,
        api_key: Option<String> = 28,
        avatar_color: Option<String> = 29,
        external_id: Option<String> = 30
    }
}

// fdb_table! {
//     users_collections
// }

fdb_table! {
    group (uuid, name) <- (name) {
        uuid: String = 0,
        organizations_uuid: String = 1,
        name: String = 2,
        access_all: bool = 3,
        external_id: Option<String> = 4,
        creation_date: bson::DateTime = 5,
        revision_date: bson::DateTime = 6
    }
}

// PostgreSQL schema for reference below
// table! {
//     users_collections (user_uuid, collection_uuid) {
//         user_uuid -> Text,
//         collection_uuid -> Text,
//         read_only -> Bool,
//         hide_passwords -> Bool,
//     }
// }
//
// table! {
//     users_organizations (uuid) {
//         uuid -> Text,
//         user_uuid -> Text,
//         org_uuid -> Text,
//         access_all -> Bool,
//         akey -> Text,
//         status -> Integer,
//         atype -> Integer,
//         reset_password_key -> Nullable<Text>,
//         external_id -> Nullable<Text>,
//     }
// }
//
// table! {
//     organization_api_key (uuid, org_uuid) {
//         uuid -> Text,
//         org_uuid -> Text,
//         atype -> Integer,
//         api_key -> Text,
//         revision_date -> Timestamp,
//     }
// }
//
// table! {
//     emergency_access (uuid) {
//         uuid -> Text,
//         grantor_uuid -> Text,
//         grantee_uuid -> Nullable<Text>,
//         email -> Nullable<Text>,
//         key_encrypted -> Nullable<Text>,
//         atype -> Integer,
//         status -> Integer,
//         wait_time_days -> Integer,
//         recovery_initiated_at -> Nullable<Timestamp>,
//         last_notification_at -> Nullable<Timestamp>,
//         updated_at -> Timestamp,
//         created_at -> Timestamp,
//     }
// }
//
// table! {
//     groups (uuid) {
//         uuid -> Text,
//         organizations_uuid -> Text,
//         name -> Text,
//         access_all -> Bool,
//         external_id -> Nullable<Text>,
//         creation_date -> Timestamp,
//         revision_date -> Timestamp,
//     }
// }
//
// table! {
//     groups_users (groups_uuid, users_organizations_uuid) {
//         groups_uuid -> Text,
//         users_organizations_uuid -> Text,
//     }
// }
//
// table! {
//     collections_groups (collections_uuid, groups_uuid) {
//         collections_uuid -> Text,
//         groups_uuid -> Text,
//         read_only -> Bool,
//         hide_passwords -> Bool,
//     }
// }
//
// table! {
//     auth_requests  (uuid) {
//         uuid -> Text,
//         user_uuid -> Text,
//         organization_uuid -> Nullable<Text>,
//         request_device_identifier -> Text,
//         device_type -> Integer,
//         request_ip -> Text,
//         response_device_id -> Nullable<Text>,
//         access_code -> Text,
//         public_key -> Text,
//         enc_key -> Nullable<Text>,
//         master_password_hash -> Nullable<Text>,
//         approved -> Nullable<Bool>,
//         creation_date -> Timestamp,
//         response_date -> Nullable<Timestamp>,
//         authentication_date -> Nullable<Timestamp>,
//     }
// }
//
// joinable!(attachments -> ciphers (cipher_uuid));
// joinable!(ciphers -> organizations (organization_uuid));
// joinable!(ciphers -> users (user_uuid));
// joinable!(ciphers_collections -> ciphers (cipher_uuid));
// joinable!(ciphers_collections -> collections (collection_uuid));
// joinable!(collections -> organizations (org_uuid));
// joinable!(devices -> users (user_uuid));
// joinable!(folders -> users (user_uuid));
// joinable!(folders_ciphers -> ciphers (cipher_uuid));
// joinable!(folders_ciphers -> folders (folder_uuid));
// joinable!(org_policies -> organizations (org_uuid));
// joinable!(sends -> organizations (organization_uuid));
// joinable!(sends -> users (user_uuid));
// joinable!(twofactor -> users (user_uuid));
// joinable!(users_collections -> collections (collection_uuid));
// joinable!(users_collections -> users (user_uuid));
// joinable!(users_organizations -> organizations (org_uuid));
// joinable!(users_organizations -> users (user_uuid));
// joinable!(users_organizations -> ciphers (org_uuid));
// joinable!(organization_api_key -> organizations (org_uuid));
// joinable!(emergency_access -> users (grantor_uuid));
// joinable!(groups -> organizations (organizations_uuid));
// joinable!(groups_users -> users_organizations (users_organizations_uuid));
// joinable!(groups_users -> groups (groups_uuid));
// joinable!(collections_groups -> collections (collections_uuid));
// joinable!(collections_groups -> groups (groups_uuid));
// joinable!(event -> users_organizations (uuid));
// joinable!(auth_requests -> users (user_uuid));
//
// allow_tables_to_appear_in_same_query!(
//     attachments,
//     ciphers,
//     ciphers_collections,
//     collections,
//     devices,
//     folders,
//     folders_ciphers,
//     invitations,
//     org_policies,
//     organizations,
//     sends,
//     twofactor,
//     users,
//     users_collections,
//     users_organizations,
//     organization_api_key,
//     emergency_access,
//     groups,
//     groups_users,
//     collections_groups,
//     event,
//     auth_requests,
// );
