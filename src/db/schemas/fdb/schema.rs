use crate::{fdb_table, fdb_key, fdb_relationship, fdb_relationship_with_attr};

fdb_table! {
    attachment (id) {
        id: String = 0,
        cipher_uuid: String = 1,
        file_name: String = 2,
        file_size: i64 = 3,
        akey: Option<String> = 4
    }
}

fdb_table! {
    cipher (uuid) {
        uuid: String = 0,
        created_at: bson::DateTime = 1,
        updated_at: bson::DateTime = 2,
        user_uuid: Option<String> = 3,
        organization_uuid: Option<String> = 4,
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

fdb_relationship! {
    collection_cipher {
        collection (uuid) <-> cipher (uuid)
    }
}

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
        created_at: bson::DateTime = 1,
        updated_at: bson::DateTime = 2,
        user_uuid: String = 3,
        name: String = 4,
        atype: i32 = 5,
        push_uuid: Option<String> = 6,
        push_token: Option<String> = 7,
        refresh_token: String = 8,
        twofactor_remember: Option<String> = 9
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

fdb_relationship! {
    favorite {
        user (uuid) <-> cipher (uuid)
    }
}

fdb_table! {
    folder (uuid) {
        uuid: String = 0,
        created_at: bson::DateTime = 1,
        updated_at: bson::DateTime = 2,
        user_uuid: String = 3,
        name: String = 4
    }
}

fdb_relationship! {
    folder_cipher {
        folder (uuid) <-> cipher (uuid)
    }
}

fdb_key!(invitation -> email: String);

fdb_table! {
    org_policy (uuid) {
        uuid: String = 0,
        org_uuid: String = 1,
        atype: i32 = 2,
        enabled: bool = 3,
        data: String = 4
    }
}

fdb_table! {
    organization (uuid) {
        uuid: String = 0,
        name: String = 1,
        billing_email: String = 2,
        private_key: Option<String> = 3,
        public_key: Option<String> = 4
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
        password_hash: Option<Vec<u8>> = 8,
        password_salt: Option<Vec<u8>> = 9,
        password_iter: Option<i32> = 10,
        max_access_count: Option<i32> = 11,
        access_count: i32 = 12,
        creation_date: bson::DateTime = 13,
        revision_date: bson::DateTime = 14,
        expiration_date: Option<bson::DateTime> = 15,
        deletion_date: bson::DateTime = 16,
        disabled: bool = 17,
        hide_email: Option<bool> = 18
    }
}

fdb_table! {
    two_factor (uuid) {
        uuid: String = 0,
        user_uuid: String = 1,
        atype: i32 = 2,
        enabled: bool = 3,
        data: String = 4,
        last_used: i64 = 5
    }
}

fdb_table! {
    two_factor_incomplete (user_uuid, device_uuid) {
        user_uuid: String = 0,
        device_uuid: String = 1,
        device_name: String = 2,
        login_time: bson::DateTime = 3,
        ip_address: String = 4
    }
}

fdb_table! {
    user (uuid) WITH UNIQUE (email) INDEX {
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
        password_hash: Vec<u8> = 11,
        salt: Vec<u8> = 12,
        password_iterations: i32 = 13,
        password_hint: Option<String> = 14,
        akey: String = 15,
        private_key: Option<String> = 16,
        public_key: Option<String> = 17,
        _totp_secret: Option<String> = 18,
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

fdb_relationship_with_attr! {
    collection_user (user_uuid, collection_uuid) (user (uuid) <-> collection (uuid)) {
        user_uuid: String = 0,
        collection_uuid: String = 1,
        read_only: bool = 2,
        hide_passwords: bool = 3
    }
}

fdb_relationship_with_attr! {
    user_organization (user_uuid, org_uuid) (user (uuid) <-> organization (uuid)) WITH UNIQUE (uuid) INDEX {
        uuid: String = 0,
        user_uuid: String = 1,
        org_uuid: String = 2,
        access_all: bool = 3,
        akey: String = 4,
        status: i32 = 5,
        atype: i32 = 6,
        reset_password_key: Option<String> = 7,
        external_id: Option<String> = 8
    }
}

fdb_table! {
    organization_api_key (uuid, org_uuid) WITH MULTI (org_uuid) INDEX {
        uuid: String = 0,
        org_uuid: String = 1,
        atype: i32 = 2,
        api_key: String = 3,
        revision_date: bson::DateTime = 4
    }
}

fdb_table! {
    emergency_access (uuid) {
        uuid: String = 0,
        grantor_uuid: String = 1,
        grantee_uuid: Option<String> = 2,
        email: Option<String> = 3,
        key_encrypted: Option<String> = 4,
        atype: i32 = 5,
        status: i32 = 6,
        wait_time_days: i32 = 7,
        recovery_initiated_at: Option<bson::DateTime> = 8,
        last_notification_at: Option<bson::DateTime> = 9,
        updated_at: bson::DateTime = 10,
        created_at: bson::DateTime = 11
    }
}

fdb_table! {
    group (uuid) {
        uuid: String = 0,
        organizations_uuid: String = 1,
        name: String = 2,
        access_all: bool = 3,
        external_id: Option<String> = 4,
        creation_date: bson::DateTime = 5,
        revision_date: bson::DateTime = 6
    }
}

fdb_relationship! {
    group_user {
        group (uuid as groups_uuid) <-> user_organization (uuid as users_organizations_uuid)
    }
}

fdb_relationship_with_attr! {
    collection_group (collections_uuid, groups_uuid) (collection (uuid) <-> group (uuid)) {
        collections_uuid: String = 0,
        groups_uuid: String = 1,
        read_only: bool = 2,
        hide_passwords: bool = 3
    }
}

fdb_table! {
    auth_request (uuid) {
        uuid: String = 0,
        user_uuid: String = 1,
        organization_uuid: Option<String> = 2,
        request_device_identifier: String = 3,
        device_type: i32 = 4,
        request_ip: String = 5,
        response_device_id: Option<String> = 6,
        access_code: String = 7,
        public_key: String = 8,
        enc_key: Option<String> = 9,
        master_password_hash: Option<String> = 10,
        approved: Option<bool> = 11,
        creation_date: bson::DateTime = 12,
        response_date: Option<bson::DateTime> = 13,
        authentication_date: Option<bson::DateTime> = 14
    }
}
