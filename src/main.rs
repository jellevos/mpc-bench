use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;
use std::fmt::Debug;

struct Message {
    from_id: usize,
    contents: Vec<u8>,
}

#[derive(Debug)]
struct Statistics {
    name: Option<String>,
    sent_bytes: Vec<usize>,
}

struct Party {
    id: usize,
    name: Option<String>,
    senders: Vec<Sender<Message>>,
    sent_bytes: Vec<usize>,
    receiver: Receiver<Message>,
    buffer: HashMap<usize, Vec<u8>>,
}

impl Party {
    fn new(id: usize, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        let sender_count = senders.len();

        Party {
            id,
            name: None,
            senders,
            sent_bytes: vec![0; sender_count],
            receiver,
            buffer: HashMap::new(),
        }
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
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

    fn send(&mut self, message: &Vec<u8>, to_party: &usize) {
        let byte_count = message.len();

        self.senders[*to_party].send(Message {
            from_id: self.id,
            contents: message.to_vec(),
        }).unwrap();

        self.sent_bytes[*to_party] += byte_count;
    }

    fn broadcast(&mut self, message: &Vec<u8>) {
        let byte_count = message.len();

        for sender in &self.senders {
            sender.send(Message {
                from_id: self.id,
                contents: message.to_vec(),
            }).unwrap();
        }

        for i in 0..self.sent_bytes.len() {
            self.sent_bytes[i] += byte_count;
        }
    }

    fn get_statistics(self) -> Statistics {
        Statistics {
            name: self.name,
            sent_bytes: self.sent_bytes,
        }
    }
}

trait Protocol<I: 'static + std::marker::Send, O: 'static + Debug + std::marker::Send> {

    fn evaluate(n_parties: usize, mut inputs: Vec<I>) {
        let mut receivers = vec![];
        let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

        for _ in 0..n_parties {
            let (sender, receiver) = channel();

            receivers.push(receiver);

            for j in 0..n_parties {
                senders[j].push(sender.clone());
            }
        }

        let handles: Vec<JoinHandle<_>> = (0..n_parties)
            .zip(receivers.drain(0..n_parties))
            .zip(senders.drain(0..n_parties))
            .zip(inputs.drain(0..n_parties))
            .map(|(((i, r), ss), input)| spawn(move ||
                Self::run_party(i, n_parties, Party::new(i, r, ss), input)))
            .collect();

        let outputs: Vec<(Statistics, O)> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        println!("{:?}", outputs);
    }

    fn run_party(id: usize, n_parties: usize, this_party: Party, input: I) -> (Statistics, O);

}

struct Example;

impl Protocol<usize, usize> for Example {
    fn run_party(id: usize, n_parties: usize, mut this_party: Party, input: usize) -> (Statistics, usize) {
        match id {
            0 => this_party.set_name(String::from("Leader")),
            _ => this_party.set_name(format!("Assistant {}", id))
        };

        println!("Hi! I am {}/{}", id, n_parties - 1);

        for i in (id + 1)..n_parties {
            this_party.send(&vec![id as u8], &i);
        }

        for j in 0..id {
            println!("I am {}/{} and I received a message from {}", id, n_parties - 1, this_party.receive(&j)[0]);
        }

        (this_party.get_statistics(), id + input)
    }
}

fn main() {
    Example::evaluate(5, vec![10; 5]);
}
