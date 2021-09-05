mod ipc;

use std::thread;

use serde_json::{Map, Value};

use self::ipc::{connect, IncomingMessage, Outgoing};

pub use self::ipc::OutgoingMessage;

pub struct Lightroom {
    state: Map<String, Value>,
    outgoing: Outgoing,
}

impl Lightroom {
    pub fn new(incoming_port: u16, outgoing_port: u16) -> Lightroom {
        let (incoming, outgoing) = connect(incoming_port, outgoing_port);

        thread::spawn(move || {
            for message in incoming {
                match message {
                    IncomingMessage::Test => (),
                    IncomingMessage::Disconnect => (),
                    IncomingMessage::State(state) => {}
                }
            }
        });

        Lightroom {
            state: Map::new(),
            outgoing,
        }
    }

    pub fn send(&self, message: OutgoingMessage) {
        self.outgoing.send(message)
    }
}
