#![feature(fs_try_exists)]

use settings::SettingsProvider;
use log::{error, info};
use tokio::signal;
use crate::mumble_actor::MumbleActorHandle;
use crate::state_file_actor::StateFileActorHandle;
use crate::telegram_bot_actor::TelegramBotActorHandle;
use crate::telegram_sender_actor::TelegramSenderActorHandle;

mod settings;
mod mumble_actor;
mod telegram_sender_actor;
mod telegram_bot_actor;
mod state_file_actor;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config = settings::Settings::get().unwrap();

    info!("{:?}", config);

    let state_file_actor_handle = StateFileActorHandle::new(&config.state_file_path);
    let telegram_sender_actor_handle = TelegramSenderActorHandle::new(&config.telegram, state_file_actor_handle);
    let (mumble_actor_handle, mumble_server_disconnected_handle) = MumbleActorHandle::new(config.mumble.clone(), telegram_sender_actor_handle.0.clone()).await;
    let _telegram_bot_actor_handle = TelegramBotActorHandle::new(config.telegram.clone(), mumble_actor_handle.clone());

    let mut core_task_handles = vec![];
    core_task_handles.push(mumble_server_disconnected_handle);
    core_task_handles.push(tokio::spawn(listen_for_sigterm()));

    info!("Mumble Telegram Bot started up");
    
    let _ = futures::future::select_all(core_task_handles).await;
}

async fn listen_for_sigterm() {
    match signal::ctrl_c().await {
        Ok(_) => {info!("App shutting down")}
        Err(err) => {error!("Unable to listen for shutdown signal: {}", err)}
    }
}