use chrono::NaiveDateTime;
use uuid::Uuid;

pub struct User {
    pub uuid: Uuid,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub verified_at: Option<NaiveDateTime>,
    pub last_verifying_at: Option<NaiveDateTime>,
    pub login_verify_count: i32,

    pub email: String,
    pub email_new: Option<String>,
    pub email_new_token: Option<String>,
    pub name: String,

    pub password_hash: Vec<u8>,
    pub salt: Vec<u8>,
    pub password_iterations: i32,
    pub password_hint: Option<String>,

    pub akey: String,
    pub private_key: Option<String>,
    pub public_key: Option<String>,

    _totp_secret: Option<String>,
    pub totp_recover: Option<String>,

    pub security_stamp: String,
    pub stamp_exception: Option<String>,

    pub equivalent_domains: String,
    pub excluded_globals: String,

    pub client_kdf: ClientKdf,

    pub api_key: Option<String>,

    pub avatar_color: Option<String>,
}

#[repr(u8)]
pub enum ClientKdf {
    Pkbdf2 {
        /// From 600000 to 2000000
        iterations: u32
    } = 1,
    Argon2id {
        /// From 2 to 10
        iterations: u8,
        /// In MB (megabytes?). From 16 to 1024
        memory: u16,
        /// From 1 to 16
        parallelism: u8,
    } = 2,
}

impl Default for ClientKdf {
    fn default() -> Self {
        Self::Pkbdf2 {
            iterations: 600000,
        }
    }
}