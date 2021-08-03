use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;

struct Message {
    from_id: usize,
    contents: Vec<u8>,
}

impl Message {
    /// Size in bytes
    fn size(&self) -> usize {
        self.contents.len()
    }
}

struct Mailbox {
    receiver: Receiver<Message>,
    buffer: HashMap<usize, Vec<u8>>,
}

impl Mailbox {
    fn new(receiver: Receiver<Message>) -> Self {
        Mailbox {
            receiver,
            buffer: HashMap::new(),
        }
    }

    fn receive(&mut self, from_id: &usize) -> Vec<u8> {
        let contents = self.buffer.remove(from_id);
        match contents {
            Some(c) => c,
            None => loop {
                let message = self.receiver.recv().unwrap();

                if message.from_id == *from_id {
                    break message.contents
                }

                self.buffer.insert(message.from_id, message.contents);
            }
        }
    }
}

struct Contacts {
    id: usize,
    senders: Vec<Sender<Message>>,
}

impl Contacts {
    fn new(id: usize, senders: Vec<Sender<Message>>) -> Self {
        Contacts {
            id,
            senders
        }
    }

    fn send(&self, message: Vec<u8>, to_party: &usize) {
        self.senders[*to_party].send(Message {
            from_id: self.id,
            contents: message,
        }).unwrap();
    }
}

fn run(id: usize, n_parties: usize, mailbox: &mut Mailbox, contacts: Contacts) {
    println!("Hi! I am {}/{}", id, n_parties - 1);
    for i in (id+1)..n_parties {
        contacts.send(vec![id as u8], &i);
    }
    for j in 0..id {
        println!("I am {}/{} and I received a message from {}", id, n_parties - 1, mailbox.receive(&j)[0]);
    }
}

fn main() {
    let n_parties = 5;

    let mut receivers = vec![];
    let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

    for _ in 0..n_parties {
        let (sender, receiver) = channel();

        receivers.push(receiver);

        for j in 0..n_parties {
            senders[j].push(sender.clone());
        }
    }

    let handles: Vec<JoinHandle<_>> = (0..n_parties).zip(receivers.drain(0..n_parties)).zip(senders.drain(0..n_parties))
        .map(|((i, r), ss)| spawn(move ||
            run(i, n_parties, &mut Mailbox::new(r), Contacts::new(i, ss))))
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
