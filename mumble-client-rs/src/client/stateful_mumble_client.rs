pub mod server;
pub mod channel;
pub mod user;
pub mod event;

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;
use std::sync::Mutex;
use log::error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use mumble_protocol_rs::control::{ControlPacket};
use crate::client::stateful_mumble_client::channel::ChannelState;
use crate::client::stateful_mumble_client::server::ServerState;
use crate::client::stateful_mumble_client::user::UserState;
use crate::{MumbleClientConfig, RawMumbleClient};
pub use crate::client::stateful_mumble_client::event::MumbleEvent;

struct State {
    server: ServerState,
    channels: HashMap<u32, ChannelState>,
    users: HashMap<u32, UserState>
}

pub struct StatefulMumbleClient {
    pub raw_client: RawMumbleClient,
    event_sender: broadcast::Sender<MumbleEvent>,
    state: Arc<Mutex<State>>
}

impl StatefulMumbleClient {
    pub async fn connect(config: &MumbleClientConfig) -> Result<(StatefulMumbleClient, JoinHandle<()>), Box<dyn Error>> {
        let (raw_client, server_connected_handle) = RawMumbleClient::connect(config).await?;

        let state = Arc::new(Mutex::new(State {
            server: ServerState::default(),
            channels: HashMap::new(),
            users: HashMap::new()
        }));

        let (mumble_event_broadcast_sender, _) = broadcast::channel(32);

        tokio::spawn(raw_client_event_handler(raw_client.subscribe(), state.clone(), mumble_event_broadcast_sender.clone()));

        Ok((StatefulMumbleClient {
            raw_client,
            event_sender: mumble_event_broadcast_sender,
            state
        }, server_connected_handle))
    }

    pub fn subscribe_to_mumble_events(&self) -> Receiver<MumbleEvent> {
        self.event_sender.subscribe()
    }

    pub fn get_current_online_users(&self) -> Vec<UserState> {
        let state = self.state.lock().unwrap();
        state.users.values().cloned().collect()
    }
}

async fn raw_client_event_handler(mut receiver: Receiver<ControlPacket>, mut state: Arc<Mutex<State>>, event_sender: broadcast::Sender<MumbleEvent>) {
    let mut startup_finished = false;
    while let Ok(packet) = receiver.recv().await {
        if matches!(&packet, ControlPacket::ServerSync(_)) {
            startup_finished = true;
        }
        for event in handle_control_packet(packet, &mut state).await {
            if startup_finished {
                if let Err(err) = event_sender.send(event) {
                    error!("Error sending mumble event: {}", err);
                }
            }
        }
    }
}

async fn handle_control_packet(packet: ControlPacket, state: &mut Arc<Mutex<State>>) -> Vec<MumbleEvent> {
    match packet {
        ControlPacket::ServerSync(s) => {
            let mut state = state.lock().unwrap();
            state.server.update_from_server_sync(*s)
        },
        ControlPacket::ServerConfig(c) => {
            let mut state = state.lock().unwrap();
            state.server.update_from_server_config(*c)
        },
        ControlPacket::Version(v) => {
            let mut state = state.lock().unwrap();
            state.server.server_info = Some(*v);
            vec![MumbleEvent::ServerStateUpdated(state.server.clone())]
        },
        ControlPacket::ChannelState(c) => {
            if c.channel_id.is_none() {
                return vec![]
            }
            let mut state = state.lock().unwrap();
            match state.channels.get_mut(&c.channel_id.unwrap()) {
                Some(channel) => channel.update_from_channel_state_packet(*c),
                None => {
                    let new_channel = ChannelState {
                        id: c.channel_id(),
                        name: c.name.unwrap_or("Root".to_string()),
                        description: c.description,
                        parent_channel_id: c.parent,
                        max_users: c.max_users
                    };
                    state.channels.insert(c.channel_id.unwrap(), new_channel.clone());
                    vec![MumbleEvent::ChannelCreated(new_channel)]
                }
            }
        },
        ControlPacket::ChannelRemove(c) => {
            let mut state = state.lock().unwrap();
            match state.channels.remove(&c.channel_id) {
                Some(channel) => vec![MumbleEvent::ChannelDeleted(channel)],
                None => vec![]
            }
        },
        ControlPacket::UserState(u) => {
            if u.session.is_none() {
                return vec![]
            }
            let mut state = state.lock().unwrap();
            match state.users.get_mut(&u.session.unwrap()) {
                Some(user) => user.update_from_user_state_packet(*u),
                None => {
                    let new_user = UserState {
                        session_id: u.session.unwrap(),
                        user_id: u.user_id,
                        name: u.name.unwrap_or("Unknown".to_string()),
                        current_channel_id: u.channel_id,
                        deafened: u.self_deaf.unwrap_or(false),
                        muted: u.self_mute.unwrap_or(false),
                    };

                    state.users.insert(u.session.unwrap(), new_user.clone());
                    vec![MumbleEvent::UserJoinedServer(new_user)]
                }
            }
        },
        ControlPacket::UserRemove(u) => {
            let mut state = state.lock().unwrap();
            match state.users.remove(&u.session) {
                Some(user) => vec![MumbleEvent::UserLeftServer(user)],
                None => vec![]
            }
        }
        ControlPacket::TextMessage(t) => {
            let state = state.lock().unwrap();
            vec![MumbleEvent::TextMessagePosted(t.message, t.actor.and_then(|actor_id| state.users.get(&actor_id)).cloned())]
        }
        _ => vec![]
    }
}
