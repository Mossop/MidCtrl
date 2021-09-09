mod actions;
mod ipc;

use std::{sync::mpsc::Sender, thread};

use crate::ControlMessage;

use self::ipc::{connect, IncomingMessage};

pub use self::actions::LightroomAction;
pub use self::ipc::OutgoingMessage;

pub struct Lightroom {
    _control_sender: Sender<ControlMessage>,
    sender: Sender<OutgoingMessage>,
}

impl Lightroom {
    pub fn new(
        control_sender: Sender<ControlMessage>,
        incoming_port: u16,
        outgoing_port: u16,
    ) -> Lightroom {
        let (incoming, sender) = connect(incoming_port, outgoing_port);

        let thread_sender = control_sender.clone();
        thread::spawn(move || {
            let send_control_message = move |message| {
                if let Err(e) = thread_sender.send(message) {
                    log::error!("Failed to send state update: {}", e);
                }
            };

            for message in incoming {
                match message {
                    IncomingMessage::Disconnect => break,
                    IncomingMessage::Test => (),
                    IncomingMessage::Reset => send_control_message(ControlMessage::Reset),
                    IncomingMessage::State { values } => {
                        send_control_message(ControlMessage::StateChange { values })
                    }
                }
            }

            send_control_message(ControlMessage::Disconnect);
        });

        Lightroom {
            _control_sender: control_sender,
            sender,
        }
    }

    pub fn send(&self, message: OutgoingMessage) {
        if let Err(e) = self.sender.send(message) {
            log::error!("Failed to send IPC message: {}", e);
        }
    }
}
