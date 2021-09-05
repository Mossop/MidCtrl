mod ipc;

use std::{sync::mpsc::Sender, thread};

use crate::{state::Module, ControlMessage};

use self::ipc::{connect, IncomingMessage};

pub use self::ipc::OutgoingMessage;

pub struct Lightroom {
    sender: Sender<OutgoingMessage>,
}

impl Lightroom {
    pub fn new(
        control_sender: Sender<ControlMessage>,
        incoming_port: u16,
        outgoing_port: u16,
    ) -> Lightroom {
        let (incoming, sender) = connect(incoming_port, outgoing_port);

        thread::spawn(move || {
            for message in incoming {
                match message {
                    IncomingMessage::Test => (),
                    IncomingMessage::Disconnect => (),
                    IncomingMessage::State(state) => {
                        if let Err(e) = control_sender
                            .send(ControlMessage::StateChange(Module::Lightroom, state))
                        {
                            log::error!("Failed to send state update: {}", e);
                        }
                    }
                }
            }
        });

        Lightroom { sender }
    }

    pub fn send(&self, message: OutgoingMessage) {
        if let Err(e) = self.sender.send(message) {
            log::error!("Failed to send IPC message: {}", e);
        }
    }
}
