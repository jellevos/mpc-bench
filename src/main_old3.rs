use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use std::thread::spawn;
use std::borrow::Borrow;

struct Message {
    from_party: u32,
    contents: Vec<u8>,
}

struct Party {
    id: u32,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    contacts: HashMap<u32, Sender<Message>>,
    buffer: HashMap<u32, Vec<u8>>
}

impl Party {
    fn new(unique_id: u32) -> Self {
        let (sender, receiver) = channel();

        Party {
            id: unique_id,
            sender,
            receiver,
            contacts: HashMap::new(),
            buffer: HashMap::new(),
        }
    }

    fn send(&self, message: Vec<u8>, to_party: &u32) {
        self.contacts[to_party].send(Message {
            from_party: self.id,
            contents: message,
        }).unwrap();
    }

    fn receive(&mut self, from_party: &u32) -> Vec<u8> {
        let contents = self.buffer.remove(&from_party);
        match contents {
            Some(c) => c,
            None => loop {
                let message = self.receiver.recv().unwrap();

                if message.from_party == *from_party {
                    break message.contents
                }

                self.buffer.insert(message.from_party, message.contents);
            }
        }
    }

    fn create_sender(&self) -> Sender<Message> {
        self.sender.clone()
    }

    fn add_contact(&mut self, id: u32, sender: Sender<Message>) {
        self.contacts.insert(id, sender);
    }
}

fn run(party: Party) {
    println!("Hi! I am {}.", party.id);
}

fn main() {
    println!("Hello, world!");

    let mut parties: [Party] = (0..5).map(|id| Party::new(id)).collect();

    for party in &parties {
        parties[0].add_contact(party.id, party.create_sender())
    }

    let handle = spawn(move || run(parties.remove(0)));
    handle.join();
}
