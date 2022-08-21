#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use std::fmt::Debug;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;
use comm::Message;

use queues::Queue;
use stats::PartyStats;

mod comm;
mod stats;

/// A `Party` that takes part in a protocol. The party has a unique `id` and is pre-loaded with
/// communication channels to and from all the other parties. A party keeps track of its own stats.
pub struct Party {
    id: usize,
    senders: Vec<Sender<Message>>,
    receiver: Receiver<Message>,
    buffer: Vec<Queue<Vec<u8>>>,
    stats: PartyStats,
}

impl Party {
    fn new(id: usize, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        let sender_count = senders.len();

        Party {
            id,
            senders,
            receiver,
            buffer: (0..sender_count - 1).map(|_| Queue::new()).collect(),
            stats: PartyStats::new(sender_count),
        }
    }

    /// Sets an actual name for a party to make the stats easier to interpret.
    pub fn set_name(&mut self, name: String) {
        self.stats.set_name(name);
    }
}

/// A multi-party computation protocol, where each party takes in an input of type `I` and computes
/// an output of type `O`. The code a party runs should be implemented in the `run_party` method.
/// The `Protocol` should implement the `Copy` trait, as the `run_party` method will be called with
/// a fresh copy of the `Protocol` (and its parameters) for each invocation.
pub trait Protocol<I: 'static + std::marker::Send, O: 'static + Debug + std::marker::Send> {
    /// Evaluates the protocol for a given number of parties `n_parties`, each with the input
    /// provided by the `inputs` field.
    fn evaluate(self, n_parties: usize, inputs: Vec<I>) -> (Vec<PartyStats>, Vec<O>)
    where
        Self: 'static + Copy + Send,
    {
        assert_eq!(n_parties, inputs.len(), "The number of parties was {} but only received {} inputs", n_parties, inputs.len());

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
                spawn(move || Self::run_party(self, i, n_parties, Party::new(i, r, ss), input))
            })
            .collect();

        let mut all_stats = vec![];
        let mut all_outputs = vec![];
        for (stats, output) in handles.into_iter().map(|h| h.join().unwrap()) {
            all_stats.push(stats);
            all_outputs.push(output);
        }

        (all_stats, all_outputs)
    }

    /// Code to run one party in the protocol. The party gets a new copy of this protocol.
    fn run_party(self, id: usize, n_parties: usize, this_party: Party, input: I)
        -> (PartyStats, O);
}

#[cfg(test)]
mod tests {
    use crate::{Party, PartyStats, Protocol};

    #[derive(Copy, Clone)]
    struct Example;

    impl Protocol<usize, usize> for Example {
        fn run_party(
            self,
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
        let (stats, outputs) = example.evaluate(5, vec![10; 5]);

        println!("stats: {:?}", stats);
        assert_eq!(outputs[0], 10);
        assert_eq!(outputs[1], 11);
        assert_eq!(outputs[2], 12);
        assert_eq!(outputs[3], 13);
        assert_eq!(outputs[4], 14);
    }
}
