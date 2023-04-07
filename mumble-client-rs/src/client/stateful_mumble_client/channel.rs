use mumble_protocol_rs::control::protobuf;
use crate::client::stateful_mumble_client::MumbleEvent;

#[derive(Clone)]
pub struct ChannelState {
    pub id: u32,
    pub parent_channel_id: Option<u32>,
    pub name: String,
    pub description: Option<String>,
    pub max_users: Option<u32>
}

impl ChannelState {
    pub fn update_from_channel_state_packet(&mut self, packet: protobuf::ChannelState) -> Vec<MumbleEvent> {
        let mut state_changed = false;
        if packet.max_users.is_some() && packet.max_users.as_ref() != self.max_users.as_ref() {
            state_changed = true;
            self.max_users = packet.max_users;
        }
        if packet.parent.is_some() && packet.parent.as_ref() != self.parent_channel_id.as_ref() {
            state_changed = true;
            self.parent_channel_id = packet.parent;
        }
        if packet.name.is_some() && packet.name.as_ref() != Some(&self.name) {
            state_changed = true;
            self.name = packet.name.unwrap();
        }
        if packet.description.is_some() && packet.description.as_ref() != self.description.as_ref() {
            state_changed = true;
            self.description = packet.description;
        }

        if state_changed {
            return vec![MumbleEvent::ChannelUpdated(self.clone())]
        }

        vec![]
    }
}