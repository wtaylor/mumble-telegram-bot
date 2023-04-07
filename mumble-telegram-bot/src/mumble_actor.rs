use tokio::sync::{oneshot, mpsc, broadcast};
use tokio::task::JoinHandle;
use mumble_client_rs::client::stateful_mumble_client::{MumbleEvent, StatefulMumbleClient};
use mumble_client_rs::client::stateful_mumble_client::MumbleEvent::{UserJoinedServer, UserLeftServer, UserUpdated};
use mumble_client_rs::client::stateful_mumble_client::user::UserState;
use crate::settings::MumbleSettings;
use crate::telegram_sender_actor::TelegramSenderActorHandle;

struct MumbleSenderActor {
    receiver: mpsc::Receiver<MumbleSenderActorMessage>,
    mumble_client: StatefulMumbleClient,
    mumble_settings: MumbleSettings
}

pub enum MumbleSenderActorMessage {
    GetActiveUsers {
        respond_to: oneshot::Sender<Vec<UserState>>,
    }
}

impl MumbleSenderActor {
    fn new(receiver: mpsc::Receiver<MumbleSenderActorMessage>, mumble_client: StatefulMumbleClient, settings: MumbleSettings) -> Self {
        MumbleSenderActor {
            receiver,
            mumble_client,
            mumble_settings: settings
        }
    }

    async fn handle_message(&mut self, msg: MumbleSenderActorMessage) {
        match msg {
            MumbleSenderActorMessage::GetActiveUsers {respond_to} => {
                let mut users = self.mumble_client.get_current_online_users();
                if self.mumble_settings.filter_out_inferred_bot_users {
                    users = users.into_iter().filter(|u| !u.infer_is_bot_user()).collect();
                }

                let _ = respond_to.send(users);
            }
        }
    }
}

async fn run_mumble_sender_actor(mut actor: MumbleSenderActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

struct MumbleEventReceiverActor {
    mumble_settings: MumbleSettings,
    mumble_event_receiver: broadcast::Receiver<MumbleEvent>,
    mumble_actor_handle: MumbleActorHandle,
    telegram_sender_actor_handle: TelegramSenderActorHandle,
}

impl MumbleEventReceiverActor {
    fn new(
        mumble_settings: MumbleSettings,
        mumble_event_receiver: broadcast::Receiver<MumbleEvent>,
        mumble_actor_handle: MumbleActorHandle,
        telegram_sender_actor_handle: TelegramSenderActorHandle) -> Self {
        MumbleEventReceiverActor {
            mumble_settings,
            mumble_event_receiver,
            mumble_actor_handle,
            telegram_sender_actor_handle
        }
    }

    async fn handle_message(&mut self, event: MumbleEvent) {
        if matches!(event, UserJoinedServer(_) | UserLeftServer(_) | UserUpdated(_)) {
            let users = self.mumble_actor_handle.get_active_users().await;
            self.telegram_sender_actor_handle.update_pinned_mumble_status_message(users).await;
        }

        match event {
            UserJoinedServer(user) => self.handle_user_joined_server_event(user).await,
            _ => {}
        }
    }

    async fn handle_user_joined_server_event(&mut self, user: UserState) {
        if self.mumble_settings.filter_out_inferred_bot_users && user.infer_is_bot_user() {
            return;
        }

        self.telegram_sender_actor_handle.send_telegram_message(format!("🎧➕ {} joined mumble", user.name)).await
    }
}

async fn run_mumble_event_receiver_actor(mut actor: MumbleEventReceiverActor) {
    while let Ok(msg) = actor.mumble_event_receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct MumbleActorHandle {
    sender: mpsc::Sender<MumbleSenderActorMessage>
}

impl MumbleActorHandle {
    pub async fn new(settings: MumbleSettings, telegram_sender_actor_handle: TelegramSenderActorHandle) -> (Self, JoinHandle<()>) {
        let (mumble_client, mumble_server_disconnected_handle) = StatefulMumbleClient::connect(&settings.clone().into()).await.unwrap();

        let (sender, receiver) = mpsc::channel(16);

        let mumble_actor_handle = Self {sender};

        let mumble_event_receiver_actor = MumbleEventReceiverActor::new(
            settings.clone(),
            mumble_client.subscribe_to_mumble_events(),
            mumble_actor_handle.clone(),
            telegram_sender_actor_handle);
        let sender_actor = MumbleSenderActor::new(receiver, mumble_client, settings.clone());
        let _sender_task = tokio::spawn(run_mumble_sender_actor(sender_actor));
        let _receiver_task = tokio::spawn(run_mumble_event_receiver_actor(mumble_event_receiver_actor));

        (mumble_actor_handle, mumble_server_disconnected_handle)
    }

    pub async fn get_active_users(&self) -> Vec<UserState> {
        let (send, recv) = oneshot::channel();
        let msg = MumbleSenderActorMessage::GetActiveUsers {
            respond_to: send
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Mumble actor has been killed")
    }
}
