use mumble_protocol_rs::control::protobuf;

pub struct MumbleClientInfo {
    os: String,
    os_version: String,
    client_release_name: String,
    client_version: u32
}

impl MumbleClientInfo {
    pub fn from_system() -> Self {
        let system_info = os_info::get();
        Self {
            os: system_info.os_type().to_string(),
            os_version: system_info.version().to_string(),
            client_release_name: "mumble-client-rs:0.0.1".to_string(),
            client_version: (1 << 16) | (5 << 8) | 18
        }
    }
}

impl Into<protobuf::Version> for MumbleClientInfo {
    fn into(self) -> protobuf::Version {
        protobuf::Version {
            os: self.os.into(),
            os_version: self.os_version.into(),
            release: self.client_release_name.into(),
            version: self.client_version.into()
        }
    }
}
