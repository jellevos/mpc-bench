#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Instant, Duration};

struct Message {
    from_id: usize,
    contents: Vec<u8>,
}

#[derive(Debug)]
pub struct PartyStats {
    name: Option<String>,
    sent_bytes: Vec<usize>,
    measured_durations: Vec<(String, Duration)>,
}

impl PartyStats {
    fn new(sender_count: usize) -> Self {
        PartyStats {
            name: None,
            sent_bytes: vec![0; sender_count],
            measured_durations: vec![],
        }
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn add_sent_bytes(&mut self, byte_count: usize, to_id: &usize) {
        self.sent_bytes[*to_id] += byte_count;
    }

    fn write_duration(&mut self, name: String, duration: Duration) {
        self.measured_durations.push((name, duration));
    }
}

pub struct Timer {
    name: String,
    start_time: Instant,
}

impl Timer {
    fn new(name: String) -> Self {
        Timer {
            name,
            start_time: Instant::now(),
        }
    }

    fn stop(&self) -> (String, Duration) {
        (self.name.clone(), self.start_time.elapsed())
    }
}

pub struct Party {
    id: usize,
    senders: Vec<Sender<Message>>,
    receiver: Receiver<Message>,
    buffer: HashMap<usize, Vec<u8>>,
    stats: PartyStats,
}

impl Party {
    fn new(id: usize, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        let sender_count = senders.len();

        Party {
            id,
            senders,
            receiver,
            buffer: HashMap::new(),
            stats: PartyStats::new(sender_count),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.stats.set_name(name);
    }

    pub fn create_timer(&self, name: &str) -> Timer {
        Timer::new(String::from(name))
    }

    pub fn stop_timer(&mut self, timer: Timer) {
        let (name, duration) = timer.stop();
        self.stats.write_duration(name, duration);
    }

    pub fn receive(&mut self, from_id: &usize) -> Vec<u8> {
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

    pub fn send(&mut self, message: &Vec<u8>, to_party: &usize) {
        let byte_count = message.len();

        self.senders[*to_party].send(Message {
            from_id: self.id,
            contents: message.to_vec(),
        }).unwrap();

        self.stats.add_sent_bytes(byte_count, to_party);
    }

    pub fn broadcast(&mut self, message: &Vec<u8>) {
        let byte_count = message.len();

        for sender in &self.senders {
            sender.send(Message {
                from_id: self.id,
                contents: message.to_vec(),
            }).unwrap();
        }

        for i in 0..self.senders.len() {
            self.stats.add_sent_bytes(byte_count, &i);
        }
    }

    pub fn get_stats(self) -> PartyStats {
        self.stats
    }
}

pub trait Protocol<I: 'static + std::marker::Send, O: 'static + Debug + std::marker::Send> {

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

        let outputs: Vec<(PartyStats, O)> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        println!("{:?}", outputs);
    }

    fn run_party(id: usize, n_parties: usize, this_party: Party, input: I) -> (PartyStats, O);

}

#[cfg(test)]
mod tests {
    use crate::{Protocol, Party, PartyStats};

    struct Example;

    impl Protocol<usize, usize> for Example {
        fn run_party(id: usize, n_parties: usize, mut this_party: Party, input: usize) -> (PartyStats, usize) {
            match id {
                0 => this_party.set_name(String::from("Leader")),
                _ => this_party.set_name(format!("Assistant {}", id))
            };

            println!("Hi! I am {}/{}", id, n_parties - 1);

            let sending_timer = this_party.create_timer("sending");
            for i in (id + 1)..n_parties {
                this_party.send(&vec![id as u8], &i);
            }
            this_party.stop_timer(sending_timer);

            for j in 0..id {
                println!("I am {}/{} and I received a message from {}", id, n_parties - 1, this_party.receive(&j)[0]);
            }

            (this_party.get_stats(), id + input)
        }
    }

    #[test]
    fn it_works() {
        Example::evaluate(5, vec![10; 5]);
    }
}
