use config::{Config, ConfigError};
use serde_derive::Deserialize;
use std::env;
use mumble_client_rs::MumbleClientConfig;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[serde(rename_all = "snake_case")]
pub struct MumbleSettings {
    pub server_address: String,
    pub server_port: u16,
    pub override_tls_server_name: Option<String>,
    #[serde(default)]
    pub insecure_disable_certificate_verification: bool,
    pub username: String,
    pub password: Option<String>,
    #[serde(default)]
    pub filter_out_inferred_bot_users: bool
}

impl Into<MumbleClientConfig> for MumbleSettings {
    fn into(self) -> MumbleClientConfig {
        MumbleClientConfig {
            server_address: self.server_address,
            server_port: self.server_port,
            override_tls_server_name: self.override_tls_server_name,
            insecure_disable_certificate_verification: self.insecure_disable_certificate_verification,
            username: self.username,
            password: self.password
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[serde(rename_all = "snake_case")]
pub struct TelegramSettings {
    pub chat_id: i64,
    pub token: String
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    pub state_file_path: String,
    pub mumble: MumbleSettings,
    pub telegram: TelegramSettings
}

impl SettingsProvider for Settings {
    fn get() -> Result<Settings, ConfigError> {
        let mut binary_path = env::current_exe().unwrap();
        binary_path.pop();
        binary_path.push("config.yaml");
        let config = Config::builder()
            .add_source(config::File::from(binary_path).required(false))
            .add_source(config::File::with_name("/var/run/mumble-telegram-bot/config.yaml").required(false))
            .add_source(config::Environment::with_prefix("MUMBLE_TG_BOT").prefix_separator("__").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}

pub trait SettingsProvider {
    fn get() -> Result<Settings, ConfigError>;
}