pub struct MumbleClientConfig {
    pub server_address: String,
    pub server_port: u16,
    pub override_tls_server_name: Option<String>,
    pub insecure_disable_certificate_verification: bool,
    pub username: String,
    pub password: Option<String>
}

impl MumbleClientConfig {
    pub fn connect_address(&self) -> String {
        format!("{}:{}", self.server_address, self.server_port)
    }
}