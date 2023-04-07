use teloxide::{Bot, RequestError};
use teloxide::types::Update;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::task::JoinHandle;
use crate::mumble_actor::MumbleActorHandle;
use crate::settings::TelegramSettings;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "The following commands are supported:")]
enum TelegramCommand {
    #[command(description = "Display this help text")]
    Help
}

async fn commands_handler(bot: Bot, msg: Message, cmd: TelegramCommand, _mumble: MumbleActorHandle) -> Result<(), RequestError> {
    match cmd {
        TelegramCommand::Help => {
            bot.send_message(msg.chat.id, TelegramCommand::descriptions().to_string()).await?;
            Ok(())
        }
    }
}

async fn run_telegram_bot_actor(settings: TelegramSettings, mumble_actor_handle: MumbleActorHandle) {
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter(|msg: Message, settings: TelegramSettings| msg.chat.id == ChatId(settings.chat_id))
                .filter_command::<TelegramCommand>()
                .endpoint(commands_handler)
        );

    Dispatcher::builder(Bot::new(settings.token.clone()), handler)
        .dependencies(dptree::deps![settings, mumble_actor_handle])
        .enable_ctrlc_handler()
        .build()
        .dispatch().await;
}

#[derive(Clone)]
pub struct TelegramBotActorHandle {

}

impl TelegramBotActorHandle {
    pub fn new(settings: TelegramSettings, mumble_actor_handle: MumbleActorHandle) -> (Self, JoinHandle<()>) {
        let actor_task = tokio::spawn(run_telegram_bot_actor(settings, mumble_actor_handle));

        (Self {}, actor_task)
    }
}