#![feature(future_join)]

pub use crate::client::{
    raw_mumble_client::RawMumbleClient,
    // stateful_mumble_client::StatefulMumbleClient,
    config::MumbleClientConfig
};

pub use mumble_protocol_rs::control::ControlPacket;
pub use mumble_protocol_rs::control::protobuf;

pub mod tls_configuration;
pub mod client;
