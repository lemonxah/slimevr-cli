use solarxr_protocol::rpc::{
    ResetRequest, ResetRequestArgs, ResetType, RpcMessage, RpcMessageHeader, RpcMessageHeaderArgs,
};
use solarxr_protocol::{MessageBundle, MessageBundleArgs};
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
    let cli = Cli::parse();
    match cli.command {
        Commands::FullReset => send_reset(ResetType::Full),
        Commands::YawReset => send_reset(ResetType::Yaw),
    }
}

fn send_reset(rtype: ResetType) {
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
    let (mut socket, _resposne) = connect("ws://localhost:21110").unwrap();
    socket.send(Message::Binary(data)).unwrap();
}
