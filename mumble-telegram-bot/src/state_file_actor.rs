use std::env;
use std::path::PathBuf;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::{oneshot, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentState {
    pub mumble_rolling_state_message_id: Option<i32>
}

struct StateFileActor {
    receiver: mpsc::Receiver<StateFileActorMessage>,
    state_file_location: PathBuf,
    state_snapshot: Option<PersistentState>
}

pub enum StateFileActorMessage {
    SaveState {
        respond_to: oneshot::Sender<()>,
        state: PersistentState
    },
    GetState {
        respond_to: oneshot::Sender<PersistentState>
    }
}

impl StateFileActor {
    fn new(receiver: mpsc::Receiver<StateFileActorMessage>, state_file_location: PathBuf) -> Self {
        Self {
            receiver,
            state_file_location,
            state_snapshot: None
        }
    }

    async fn handle_message(&mut self, msg: StateFileActorMessage) {
        match msg {
            StateFileActorMessage::GetState {respond_to} => {
                if let Some(state_snapshot) = self.state_snapshot.as_ref() {
                    respond_to.send(state_snapshot.clone()).unwrap();
                    return;
                }

                if std::fs::try_exists(&self.state_file_location).is_ok_and(|exists| exists) {
                    let state = std::fs::read_to_string(&self.state_file_location).unwrap();
                    let state = serde_json::from_str::<PersistentState>(&state)
                        .expect("State is corrupted and cannot be deserialised");
                    respond_to.send(state).unwrap();
                }

                else {
                    let state = PersistentState {
                        mumble_rolling_state_message_id: None
                    };
                    self.state_snapshot = Some(state.clone());
                    std::fs::write(&self.state_file_location, serde_json::to_string_pretty(&state).unwrap()).unwrap();
                    respond_to.send(state).unwrap();
                }
            },
            StateFileActorMessage::SaveState {respond_to, state} => {
                std::fs::write(&self.state_file_location, serde_json::to_string_pretty(&state).unwrap()).unwrap();
                self.state_snapshot = Some(state);
                respond_to.send(()).unwrap();
            }
        }
    }
}

fn resolve_absolute_path(path: &str) -> PathBuf {
    let mut path = PathBuf::from(path);
    if !path.is_absolute() {
        let mut base_path = env::current_exe().unwrap();
        base_path.pop();
        base_path.push(path);
        path = base_path;
    }

    path
}

async fn run_actor(mut actor: StateFileActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct StateFileActorHandle {
    sender: mpsc::Sender<StateFileActorMessage>
}

impl StateFileActorHandle {
    pub fn new(state_file_path: &str) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = StateFileActor::new(receiver, resolve_absolute_path(state_file_path));
        let _ = tokio::spawn(run_actor(actor));

        Self {sender}
    }

    pub async fn get_state(&self) -> PersistentState {
        let (send, recv) = oneshot::channel();
        let msg = StateFileActorMessage::GetState {
            respond_to: send
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor has been killed")
    }

    pub async fn save_state(&self, state: PersistentState) {
        let (send, recv) = oneshot::channel();
        let msg = StateFileActorMessage::SaveState {
            respond_to: send,
            state
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor has been killed");
    }
}