mod ipc;

use std::{sync::mpsc::Sender, thread};

use crate::{state::Module, ControlMessage};

use self::ipc::{connect, IncomingMessage, Outgoing};

pub use self::ipc::OutgoingMessage;

pub struct Lightroom {
    outgoing: Outgoing,
}

impl Lightroom {
    pub fn new(
        sender: Sender<ControlMessage>,
        incoming_port: u16,
        outgoing_port: u16,
    ) -> Lightroom {
        let (incoming, outgoing) = connect(incoming_port, outgoing_port);

        thread::spawn(move || {
            for message in incoming {
                match message {
                    IncomingMessage::Test => (),
                    IncomingMessage::Disconnect => (),
                    IncomingMessage::State(state) => {
                        if let Err(e) =
                            sender.send(ControlMessage::StateChange(Module::Lightroom, state))
                        {
                            log::error!("Failed to send state update: {}", e);
                        }
                    }
                }
            }
        });

        Lightroom { outgoing }
    }

    pub fn send(&self, message: OutgoingMessage) {
        self.outgoing.send(message)
    }
}
