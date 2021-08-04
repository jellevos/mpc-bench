#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Instant, Duration};

/// A message that is sent from the party with id `from_id` to another, containing a `Vec` of bytes.
struct Message {
    from_id: usize,
    contents: Vec<u8>,
}

/// Statistics pertaining to one party, such as the number of bytes sent and the durations measured.
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

/// A `Timer` that starts measuring a duration upon creation, until it is stopped.
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

/// A `Party` that takes part in a protocol. The party has a unique `id` and is pre-loaded with
/// communication channels to and from all the other parties. A party keeps track of its own stats.
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

    /// Sets an actual name for a party to make the stats easier to interpret.
    pub fn set_name(&mut self, name: String) {
        self.stats.set_name(name);
    }

    /// Creates a timer with the given `name` that starts running immediately.
    pub fn create_timer(&self, name: &str) -> Timer {
        Timer::new(String::from(name))
    }

    /// Stops the `timer` and writes it measured duration to this party's statistics.
    pub fn stop_timer(&mut self, timer: Timer) {
        let (name, duration) = timer.stop();
        self.stats.write_duration(name, duration);
    }

    /// Blocks until this party receives a message from the party with `from_id`. A message is a
    /// vector of bytes `Vec<u8>`. This can be achieved for example using `bincode` serialization.
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

    /// Sends a vector of bytes to the party with `to_id` and keeps track of the number of bits sent
    /// to this party.
    pub fn send(&mut self, message: &Vec<u8>, to_id: &usize) {
        let byte_count = message.len();

        self.senders[*to_id].send(Message {
            from_id: self.id,
            contents: message.to_vec(),
        }).unwrap();

        self.stats.add_sent_bytes(byte_count, to_id);
    }

    /// Broadcasts a message (a vector of bytes) to all parties and keeps track of the number of
    /// bits sent.
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

    /// Gets the collected statistics for this party.
    pub fn get_stats(self) -> PartyStats {
        self.stats
    }
}

/// A multi-party computation protocol, where each party takes in an input of type `I` and computes
/// an output of type `O`. The code a party runs should be implemented in the `run_party` method.
pub trait Protocol<I: 'static + std::marker::Send, O: 'static + Debug + std::marker::Send> {

    /// Evaluates the protocol for a given number of parties `n_parties`, each with the input
    /// provided by the `inputs` field.
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

    /// Code to run one party in the protocol.
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
