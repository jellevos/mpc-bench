#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;
use std::time::{Duration, Instant};

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
        // TODO: Assert length equals receivers
        let sender_count = senders.len();

        Party {
            id,
            senders,
            receiver,
            buffer: HashMap::new(), // TODO: Change to Vec
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
                    break message.contents;
                }

                self.buffer.insert(message.from_id, message.contents);
            },
        }
    }

    /// Sends a vector of bytes to the party with `to_id` and keeps track of the number of bits sent
    /// to this party.
    pub fn send(&mut self, message: &[u8], to_id: &usize) {
        let byte_count = message.len();

        self.senders[*to_id]
            .send(Message {
                from_id: self.id,
                contents: message.to_vec(),
            })
            .unwrap();

        self.stats.add_sent_bytes(byte_count, to_id);
    }

    /// Broadcasts a message (a vector of bytes) to all parties and keeps track of the number of
    /// bits sent.
    pub fn broadcast(&mut self, message: &[u8]) {
        let byte_count = message.len();

        for sender in &self.senders {
            sender
                .send(Message {
                    from_id: self.id,
                    contents: message.to_vec(),
                })
                .unwrap();
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
/// The `Protocol` should implement the `Copy` trait, as the `run_party` method will be called with
/// a fresh copy of the `Protocol` (and its parameters) for each invocation.
pub trait Protocol<I: 'static + std::marker::Send, O: 'static + Debug + std::marker::Send> {
    /// Evaluates the protocol for a given number of parties `n_parties`, each with the input
    /// provided by the `inputs` field.
    fn evaluate(self, n_parties: usize, inputs: Vec<I>) -> Vec<(PartyStats, O)>
    where
        Self: 'static + Copy + Send,
    {
        // TODO: Assert that n_parties agrees with the number of inputs
        let mut receivers = vec![];
        let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

        for _ in 0..n_parties {
            let (sender, receiver) = channel();

            receivers.push(receiver);

            for sender_vec in senders.iter_mut() {
                sender_vec.push(sender.clone());
            }
        }

        #[allow(clippy::needless_collect)]
        let handles: Vec<_> = (0..n_parties)
            .zip(receivers.into_iter())
            .zip(senders.into_iter())
            .zip(inputs.into_iter())
            .map(|(((i, r), ss), input)| {
                spawn(move || Self::run_party(&self, i, n_parties, Party::new(i, r, ss), input))
            })
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    }

    /// Code to run one party in the protocol. The party gets a new copy of this protocol.
    fn run_party(&self, id: usize, n_parties: usize, this_party: Party, input: I)
        -> (PartyStats, O);
}

#[cfg(test)]
mod tests {
    use crate::{Party, PartyStats, Protocol};

    #[derive(Copy, Clone)]
    struct Example;

    impl Protocol<usize, usize> for Example {
        fn run_party(
            &self,
            id: usize,
            n_parties: usize,
            mut this_party: Party,
            input: usize,
        ) -> (PartyStats, usize) {
            match id {
                0 => this_party.set_name(String::from("Leader")),
                _ => this_party.set_name(format!("Assistant {}", id)),
            };

            println!("Hi! I am {}/{}", id, n_parties - 1);

            let sending_timer = this_party.create_timer("sending");
            for i in (id + 1)..n_parties {
                this_party.send(&vec![id as u8], &i);
            }
            this_party.stop_timer(sending_timer);

            for j in 0..id {
                println!(
                    "I am {}/{} and I received a message from {}",
                    id,
                    n_parties - 1,
                    this_party.receive(&j)[0]
                );
            }

            (this_party.get_stats(), id + input)
        }
    }

    #[test]
    fn it_works() {
        let example = Example;
        let outputs = example.evaluate(5, vec![10; 5]);

        assert_eq!(outputs[0].1, 10);
        assert_eq!(outputs[1].1, 11);
        assert_eq!(outputs[2].1, 12);
        assert_eq!(outputs[3].1, 13);
        assert_eq!(outputs[4].1, 14);
    }
}
