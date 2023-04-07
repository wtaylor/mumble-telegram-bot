use mumble_protocol_rs::control::protobuf;
use crate::client::stateful_mumble_client::MumbleEvent;

#[derive(Clone)]
pub struct UserState {
    pub session_id: u32,
    pub user_id: Option<u32>,
    pub current_channel_id: Option<u32>,
    pub name: String,
    pub muted: bool,
    pub deafened: bool
}

impl UserState {
    pub fn infer_is_bot_user(&self) -> bool {
        self.name.ends_with("Bot")
    }

    pub fn update_from_user_state_packet(&mut self, packet: protobuf::UserState) -> Vec<MumbleEvent> {
        let mut entity_events = vec![];
        let mut state_changed = false;
        if packet.user_id.is_some() && packet.user_id.as_ref() != self.user_id.as_ref() {
            state_changed = true;
            self.user_id = packet.user_id;
        }
        if packet.channel_id.is_some() && packet.channel_id.as_ref() != self.current_channel_id.as_ref() {
            state_changed = true;
            self.current_channel_id = packet.channel_id;
            entity_events.push(MumbleEvent::UserSwitchedChannel(self.clone()))
        }
        if packet.name.is_some() && packet.name.as_ref() != Some(&self.name) {
            state_changed = true;
            self.name = packet.name.unwrap();
        }
        if packet.self_mute.is_some() && packet.self_mute.as_ref() != Some(&self.muted) {
            state_changed = true;
            self.muted = packet.self_mute.unwrap();
        }
        if packet.self_deaf.is_some() && packet.self_deaf != Some(self.deafened) {
            state_changed = true;
            self.deafened = packet.self_deaf.unwrap();
        }

        if state_changed {
            entity_events.push(MumbleEvent::UserUpdated(self.clone()))
        }

        entity_events
    }
}