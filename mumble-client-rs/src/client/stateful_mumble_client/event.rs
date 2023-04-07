use crate::client::stateful_mumble_client::channel::ChannelState;
use crate::client::stateful_mumble_client::server::ServerState;
use crate::client::stateful_mumble_client::user::UserState;

#[derive(Clone)]
pub enum MumbleEvent {
    ServerStateUpdated(ServerState),
    UserJoinedServer(UserState),
    UserLeftServer(UserState),
    UserSwitchedChannel(UserState),
    UserUpdated(UserState),
    ChannelCreated(ChannelState),
    ChannelUpdated(ChannelState),
    ChannelDeleted(ChannelState),
    TextMessagePosted(String, Option<UserState>),
}
