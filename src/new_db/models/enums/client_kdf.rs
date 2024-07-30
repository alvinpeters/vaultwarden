#[repr(u8)]
#[derive(Copy, Clone)]
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