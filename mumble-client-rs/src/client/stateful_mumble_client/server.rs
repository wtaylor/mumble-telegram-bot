use mumble_protocol_rs::control::protobuf;
use mumble_protocol_rs::control::protobuf::Version;
use crate::client::stateful_mumble_client::MumbleEvent;

pub type ServerInfo = Version;

#[derive(Default, Clone)]
pub struct ServerState {
    pub server_info: Option<ServerInfo>,
    pub user_session_id: Option<u32>,
    pub max_bandwidth: Option<u32>,
    pub welcome_text: Option<String>,
    pub allow_html: Option<bool>,
    pub max_message_length: Option<u32>,
    pub max_image_message_length: Option<u32>,
    pub max_users: Option<u32>,
    pub startup_finished: bool
}

impl ServerState {
    pub fn update_from_server_sync(&mut self, packet: protobuf::ServerSync) -> Vec<MumbleEvent> {
        self.welcome_text = packet.welcome_text;
        self.max_bandwidth = packet.max_bandwidth;
        self.user_session_id = packet.session;
        self.startup_finished = true;
        
        vec![]
    }

    pub fn update_from_server_config(&mut self, packet: protobuf::ServerConfig) -> Vec<MumbleEvent> {
        let mut state_changed = false;
        if packet.max_bandwidth.is_some() && packet.max_bandwidth.as_ref() != self.max_bandwidth.as_ref() {
            state_changed = true;
            self.max_bandwidth = packet.max_bandwidth;
        }

        if packet.welcome_text.is_some() && packet.welcome_text.as_ref() != self.welcome_text.as_ref() {
            state_changed = true;
            self.welcome_text = packet.welcome_text;
        }

        if packet.allow_html.is_some() && packet.allow_html.as_ref() != self.allow_html.as_ref() {
            state_changed = true;
            self.allow_html = packet.allow_html;
        }

        if packet.image_message_length.is_some() && packet.image_message_length.as_ref() != self.max_image_message_length.as_ref() {
            state_changed = true;
            self.max_image_message_length = packet.image_message_length;
        }

        if packet.max_users.is_some() && packet.max_users.as_ref() != self.max_users.as_ref() {
            state_changed = true;
            self.max_users = packet.max_users;
        }

        if packet.message_length.is_some() && packet.message_length.as_ref() != self.max_message_length.as_ref() {
            state_changed = true;
            self.max_message_length = packet.message_length;
        }

        if state_changed {
            return vec![MumbleEvent::ServerStateUpdated(self.clone())]
        }

        vec![]
    }
}