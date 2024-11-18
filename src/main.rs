use std::fs::File;
use std::thread;
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

fn play_mp3(file_path: &str) {
    let file = File::open(file_path).unwrap();
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();
    sink.append(rodio::Decoder::new(std::io::BufReader::new(file)).unwrap());
    sink.sleep_until_end();
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
    let prefix = if cfg!(debug_assertions) {
        ""
    } else {
        "/usr/share"
    };
    let file_path = match rtype {
        ResetType::Full => "assets/full-reset.mp3",
        ResetType::Yaw => "assets/yaw-reset.mp3",
        _ => "",
    };
    let file_path = format!("{}/{}", prefix, file_path);
    let handle = thread::spawn(move || {
        play_mp3(&file_path);
    });
    if rtype == ResetType::Full {
        thread::sleep(Duration::from_secs(2));
    }
    info!("sending reset: {:?}", rtype);
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

    if let Ok((mut socket, _response)) = connect("ws://localhost:21110") {
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
                    //info!("Received: {}", message);
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
        handle.join().unwrap();
        info!("{:?} reset done", rtype);
        socket.close(Some(close_frame))
    } else {
        error!("Make sure that the SlimeVR server is running before running this command.");
        Ok(())
    }
}
