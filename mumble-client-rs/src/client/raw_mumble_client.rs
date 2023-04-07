use std::error::Error;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{ClientConfig, ServerName};
use tokio_rustls::TlsConnector;
use mumble_protocol_rs::control::{ControlCodec, ControlPacket, protobuf};
use tokio_util::codec::{Decoder, Framed};
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_rustls::client::TlsStream;
use crate::client::client_info::MumbleClientInfo;
use crate::MumbleClientConfig;
use crate::tls_configuration::{create_root_certificate_store, NoCertificateVerification};

pub struct RawMumbleClient {
    server_packet_broadcast_sender: broadcast::Sender<ControlPacket>,
    client_packet_sender: mpsc::Sender<ControlPacket>
}

impl RawMumbleClient {
    pub async fn connect(config: &MumbleClientConfig) -> Result<(RawMumbleClient, JoinHandle<()>), Box<dyn Error>> {
        let (mut sink, stream) = establish_tls_connection(config).await.unwrap().split();
        exchange_version_info(&mut sink).await;
        authenticate_with_server(config, &mut sink).await;
        mute_and_deafen(&mut sink).await;

        let (server_packet_broadcast_sender, _) = broadcast::channel(32);
        let (client_packet_sender, client_packet_receiver) = mpsc::channel(32);

        let _client_packet_handler = task::spawn(process_client_packets(client_packet_receiver, sink));
        let _server_packet_handler = task::spawn(broadcast_server_packets(server_packet_broadcast_sender.clone(), stream));
        let _ping_server_on_interval = task::spawn(ping_server_on_interval(10, client_packet_sender.clone()));

        Ok((Self { server_packet_broadcast_sender, client_packet_sender }, _client_packet_handler))
    }

    pub fn get_sender(&self) -> mpsc::Sender<ControlPacket> {
        self.client_packet_sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ControlPacket> {
        self.server_packet_broadcast_sender.subscribe()
    }
}

async fn ping_server_on_interval(interval: u64, packet_sender: mpsc::Sender<ControlPacket>) {
    let mut interval = time::interval(Duration::from_secs(interval));
    loop {
        interval.tick().await;
        let ping_packet = protobuf::Ping {
            timestamp: get_unix_timestamp().into(),
            ..Default::default()
        };
        debug!("Sending scheduled ping packet to server");
        packet_sender.send(ping_packet.into()).await.unwrap();
    }
}

async fn establish_tls_connection(config: &MumbleClientConfig) -> Result<Framed<TlsStream<TcpStream>, ControlCodec>, Box<dyn Error>> {
    let mut tls_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(create_root_certificate_store().unwrap())
        .with_no_client_auth();

    if config.insecure_disable_certificate_verification {
        tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification {}));
    }

    let dns_name = match &config.override_tls_server_name {
        Some(server_name) => ServerName::try_from(server_name.clone().as_str()),
        None => ServerName::try_from(config.server_address.clone().as_str())
    }.unwrap();

    info!("Connecting to mumble server: {}", config.connect_address());

    let connector = TlsConnector::from(Arc::new(tls_config));
    let tcp_stream = TcpStream::connect(config.connect_address().as_str()).await?;
    let tls_stream = connector.connect(dns_name, tcp_stream).await?;

    info!("TLS connection established to mumble server");

    Ok(ControlCodec::new().framed(tls_stream))
}

async fn exchange_version_info(sink: &mut SplitSink<Framed<TlsStream<TcpStream>, ControlCodec>, ControlPacket>) {
    info!("Exchanging version information");
    let client_info: protobuf::Version = MumbleClientInfo::from_system().into();
    sink.send(client_info.into()).await.unwrap();
}

async fn authenticate_with_server(config: &MumbleClientConfig, sink: &mut SplitSink<Framed<TlsStream<TcpStream>, ControlCodec>, ControlPacket>) {
    info!("Authenticating with server");
    let client_authentication_message = protobuf::Authenticate {
        opus: Some(true),
        username: config.username.clone().into(),
        celt_versions: Vec::new(),
        password: config.password.clone(),
        tokens: Vec::new()
    };
    sink.send(client_authentication_message.into()).await.unwrap();
}

async fn process_client_packets(mut packet_receiver: mpsc::Receiver<ControlPacket>, mut sink: SplitSink<Framed<TlsStream<TcpStream>, ControlCodec>, ControlPacket>) {
    while let Some(packet) = packet_receiver.recv().await {
        debug!("Sending Packet: {:?}", packet);

        if let Err(error) = sink.send(packet).await {
            match error.kind() {
                ErrorKind::BrokenPipe => {
                    error!("Connection to mumble server broken. Client Stopping. {}", error);
                    break;
                }
                _ => error!("Failed to send packet to mumble server: {}", error)
            }
        }
    }
}

async fn broadcast_server_packets(packet_broadcaster: broadcast::Sender<ControlPacket>, mut stream: SplitStream<Framed<TlsStream<TcpStream>, ControlCodec>>) {
    loop {
        let stream_item = stream.next().await;
        match stream_item {
            None => {
                warn!("Server connection closed, no more packets will be received");
                return;
            }
            Some(Ok(packet)) => {
                debug!("Received Packet: {:?}", packet);
                if let Err(send_error) = packet_broadcaster.send(packet) {
                    error!("Error broadcasting packet to receivers: {}", send_error);
                }
            },
            Some(Err(e)) => {
                error!("Unexpected error parsing packet: {}", e);
            }
        }
    }
}

async fn mute_and_deafen(sink: &mut SplitSink<Framed<TlsStream<TcpStream>, ControlCodec>, ControlPacket>) {
    info!("Muting and deafening bot user");
    let bot_user_state_packet = protobuf::UserState {
        self_mute: true.into(),
        self_deaf: true.into(),
        ..Default::default()
    };
    sink.send(bot_user_state_packet.into()).await.unwrap();
}

fn get_unix_timestamp() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}