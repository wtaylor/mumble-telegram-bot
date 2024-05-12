use log::debug;
use tokio::sync::{oneshot, mpsc};
use teloxide::{Bot, RequestError};
use teloxide::prelude::*;
use teloxide::types::{MessageId, Recipient};
use tokio::task::JoinHandle;
use mumble_client_rs::client::stateful_mumble_client::user::UserState;
use crate::settings::TelegramSettings;
use crate::state_file_actor::{PersistentState, StateFileActorHandle};

struct TelegramSenderActor {
    receiver: mpsc::Receiver<TelegramSenderActorMessage>,
    state_file_actor_handle: StateFileActorHandle,
    teloxide_bot: Bot,
    telegram_chat_id: i64,
    pinned_mumble_status_message: Option<i32>
}

pub enum TelegramSenderActorMessage {
    SendTelegramMessage {
        respond_to: oneshot::Sender<()>,
        message: String
    },
    UpdatePinnedMumbleStatusMessage {
        respond_to: oneshot::Sender<()>,
        active_users: Vec<UserState>
    }
}

impl TelegramSenderActor {
    fn new(
        receiver: mpsc::Receiver<TelegramSenderActorMessage>,
        state_file_actor_handle: StateFileActorHandle,
        settings: &TelegramSettings) -> Self {
        TelegramSenderActor {
            receiver,
            state_file_actor_handle,
            teloxide_bot: Bot::new(&settings.token),
            telegram_chat_id: settings.chat_id,
            pinned_mumble_status_message: None
        }
    }

    async fn load_state(&mut self) {
        let state = self.state_file_actor_handle.get_state().await;
        self.pinned_mumble_status_message = state.mumble_rolling_state_message_id;
    }

    async fn create_pinned_mumble_status_message(&mut self) {
        let message: Message = self.teloxide_bot.send_message(
            Recipient::Id(ChatId(self.telegram_chat_id)),
            "🎧 Mumble: 0 users online").await.unwrap();

        self.teloxide_bot.pin_chat_message(message.chat.id, message.id).await.unwrap();
        self.pinned_mumble_status_message = Some(message.id.0);
        self.state_file_actor_handle.save_state(PersistentState {mumble_rolling_state_message_id: Some(message.id.0)}).await;
    }

    async fn handle_message(&mut self, msg: TelegramSenderActorMessage) {
        match msg {
            TelegramSenderActorMessage::SendTelegramMessage {respond_to, message} => {
                debug!("Sending Message to configured channel: {}", message);
                let _send_result: Result<Message, RequestError> =
                    self.teloxide_bot.send_message(
                        Recipient::Id(ChatId(self.telegram_chat_id)),
                        message).await;
                let _ = respond_to.send(());
            },
            TelegramSenderActorMessage::UpdatePinnedMumbleStatusMessage {
                respond_to, active_users
            } => {
                let mut message = "🎧 Mumble: 0 users online".to_string();
                if !active_users.is_empty() {
                    let user_count = active_users.len();
                    let user_list = active_users.into_iter().map(|u| u.name).collect::<Vec<_>>().join(", ");
                    message = format!("🎧 Mumble: {} users online ({})", user_count, user_list);
                }

                match self.pinned_mumble_status_message {
                    Some(message_id) => {
                        let _ = self.teloxide_bot.edit_message_text(
                            Recipient::Id(ChatId(self.telegram_chat_id)),
                            MessageId(message_id),
                            message).await;
                    },
                    None => {
                        let _ = self.teloxide_bot.send_message(
                            Recipient::Id(ChatId(self.telegram_chat_id)),
                            message).await;
                    }
                }

                let _ = respond_to.send(());
            }
        }
    }
}

async fn run_telegram_sender_actor(mut actor: TelegramSenderActor) {
    actor.load_state().await;
    if actor.pinned_mumble_status_message.is_none() {
        actor.create_pinned_mumble_status_message().await;
    }
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct TelegramSenderActorHandle {
    sender: mpsc::Sender<TelegramSenderActorMessage>
}

impl TelegramSenderActorHandle {
    pub fn new(bot_settings: &TelegramSettings, state_file_actor_handle: StateFileActorHandle) -> (Self, JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel(32);
        let actor = TelegramSenderActor::new(receiver, state_file_actor_handle, bot_settings);
        let actor_task = tokio::spawn(run_telegram_sender_actor(actor));

        (Self {sender}, actor_task)
    }

    pub async fn send_telegram_message(&self, message: String) {
        let (send, recv) = oneshot::channel();
        let msg = TelegramSenderActorMessage::SendTelegramMessage {
            respond_to: send,
            message
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor has been killed");
    }

    pub async fn update_pinned_mumble_status_message(&self, active_users: Vec<UserState>) {
        let (send, recv) = oneshot::channel();
        let msg = TelegramSenderActorMessage::UpdatePinnedMumbleStatusMessage {
            respond_to: send,
            active_users
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor has been killed");
    }
}