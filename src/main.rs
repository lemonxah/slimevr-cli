use std::env;
use std::time::Duration;

use env_logger::Builder;
use log::{error, info, trace};
use solarxr_protocol::rpc::{
    ResetRequest, ResetRequestArgs, ResetType, RpcMessage, RpcMessageHeader, RpcMessageHeaderArgs,
};
use solarxr_protocol::{MessageBundle, MessageBundleArgs};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{connect, Message};

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Commands {
    FullReset,
    YawReset,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    Builder::new().filter(None, log::LevelFilter::Info).init();

    let cli = Cli::parse();
    if let Err(err) = match cli.command {
        Commands::FullReset => send_reset(ResetType::Full),
        Commands::YawReset => send_reset(ResetType::Yaw),
    } {
        println!("Error sending reset: {:?}", err);
    }
}

fn send_reset(rtype: ResetType) -> Result<(), tungstenite::Error> {
    let mut fbb = flatbuffers::FlatBufferBuilder::new();
    let args = RpcMessageHeaderArgs {
        tx_id: None,
        message_type: RpcMessage::ResetRequest,
        message: Some(
            ResetRequest::create(&mut fbb, &ResetRequestArgs { reset_type: rtype })
                .as_union_value(),
        ),
    };
    let header = RpcMessageHeader::create(&mut fbb, &args);
    let messages = fbb.create_vector(&[header]);
    let message = MessageBundle::create(
        &mut fbb,
        &MessageBundleArgs {
            rpc_msgs: Some(messages),
            data_feed_msgs: None,
            pub_sub_msgs: None,
        },
    );

    fbb.finish(message, None);
    let data = fbb.finished_data().to_vec();

    let (mut socket, _response) = connect("ws://localhost:21110").unwrap();
    if let Err(err) = socket.send(Message::Binary(data)) {
        error!("Error sending message: {:?}", err);
    }

    loop {
        if let Ok(msg) = socket.read() {
            if let Ok(message) = msg.to_text() {
                if message.is_empty() {
                    trace!("empty message");
                    break;
                }
                info!("Received: {}", message);
            } else {
                info!("done");
                break;
            }
        } else {
            info!("unable to read socket");
            break;
        }
    }

    let close_frame = CloseFrame {
        code: CloseCode::Normal,
        reason: "done resetting".into(),
    };
    socket.close(Some(close_frame))
}
