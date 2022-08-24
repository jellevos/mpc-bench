#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use comm::{Message, NetworkDescription, Channels};
use std::fmt::Debug;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;
use std::time::Duration;

use queues::Queue;
use stats::{PartyStats, AggregatedStats};

/// Communication module, allows parties to send and receive messages.
pub mod comm;

/// Statistics module, allows parties to track timings and bandwidth costs.
pub mod stats;

/// A `Party` that takes part in a protocol. The party will receive a unique `id` when it is running the protocol, as well as
/// communication channels to and from all the other parties. A party keeps track of its own stats.
pub trait Party {
    type Input;
    type Output: Debug + Send;

    fn get_name(&self, id: usize) -> String {
        format!("Party {}", id)
    }

    fn run(&mut self, id: usize, n_parties: usize, input: Self::Input, channels: Channels, stats: &mut PartyStats) -> Self::Output;
}

pub trait Protocol: Debug {
    type Party: Party;

    fn setup_parties(&self, n_parties: usize) -> Vec<Self::Party>;

    fn generate_inputs(&self, n_parties: usize) -> Vec<<Self::Party as Party>::Input>;

    fn validate_outputs(&self, outputs: Vec<<Self::Party as Party>::Output>) -> bool {
        true
    }

    fn evaluate<N: NetworkDescription>(&self, name: String, n_parties: usize, network_description: &N, stats: AggregatedStats, repetitions: usize) -> AggregatedStats {
        let parties = self.setup_parties(n_parties);
        debug_assert_eq!(parties.len(), n_parties);

        for _ in 0..repetitions {
            let inputs = self.generate_inputs(n_parties);
            debug_assert_eq!(inputs.len(), n_parties);

            let channels = network_description.instantiate(n_parties);
            debug_assert_eq!(channels.len(), n_parties);

            let party_stats: Vec<PartyStats> = (0..n_parties).map(|_| PartyStats::new()).collect();

            let handles = parties.iter_mut().enumerate().zip(inputs).zip(channels).zip(party_stats.iter_mut()).map(|((((id, party), input), channel), s)| spawn(move || {
                    party.run(id, n_parties, input, channel, s)
                }));

            let outputs = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
            if !self.validate_outputs(outputs) {
                println!("The outputs are invalid:\n{:?} ...for these parameters:\n{:?}", outputs, self);
                // TODO: Mark invalid in stats
            }

            for s in party_stats {
                stats.incorporate_party_stats(s);
            }
        }

        stats
    }
}


// pub struct Party {
//     id: usize,
//     stats: PartyStats,
//     latency: Duration,
//     seconds_per_byte: Duration,
// }

// impl Party {
//     fn without_communication_overhead(
//         id: usize,
//         receiver: Receiver<Message>,
//         senders: Vec<Sender<Message>>,
//     ) -> Self {
//         let sender_count = senders.len();

//         Party {
//             id,
//             senders,
//             receiver,
//             buffer: (0..sender_count - 1).map(|_| Queue::new()).collect(),
//             stats: PartyStats::new(sender_count),
//             latency: Duration::new(0, 0),
//             seconds_per_byte: Duration::new(0, 0),
//         }
//     }

//     fn with_communication_overhead(
//         id: usize,
//         receiver: Receiver<Message>,
//         senders: Vec<Sender<Message>>,
//         latency: Duration,
//         bytes_per_seconds: f64,
//     ) -> Self {
//         let sender_count = senders.len();

//         Party {
//             id,
//             senders,
//             receiver,
//             buffer: (0..sender_count - 1).map(|_| Queue::new()).collect(),
//             stats: PartyStats::new(sender_count),
//             latency,
//             seconds_per_byte: Duration::from_secs_f64(1. / bytes_per_seconds),
//         }
//     }

//     /// Sets an actual name for a party to make the stats easier to interpret.
//     pub fn set_name(&mut self, name: String) {
//         self.stats.set_name(name);
//     }
// }

/// A multi-party computation protocol, where each party takes in an input of type `I` and computes
/// an output of type `O`. The code a party runs should be implemented in the `run_party` method.
/// The `Protocol` should implement the `Copy` trait, as the `run_party` method will be called with
/// a fresh copy of the `Protocol` (and its parameters) for each invocation.
// pub trait Protocol<
//     I: 'static + std::marker::Send,
//     O: 'static + Debug + std::marker::Send,
//     S: std::marker::Send + 'static,
// >
// {
//     /// Evaluates the protocol for a given number of parties `n_parties`, each with the input
//     /// provided by the `inputs` field.
//     fn evaluate(
//         self,
//         n_parties: usize,
//         inputs: Vec<I>,
//         party_secrets: Vec<S>,
//     ) -> (Vec<PartyStats>, Vec<O>)
//     where
//         Self: 'static + Copy + Send,
//     {
//         assert_eq!(
//             n_parties,
//             inputs.len(),
//             "The number of parties was {} but only received {} inputs",
//             n_parties,
//             inputs.len()
//         );

        

//         #[allow(clippy::needless_collect)]
//         let handles: Vec<_> = (0..n_parties)
//             .zip(receivers.into_iter())
//             .zip(senders.into_iter())
//             .zip(inputs.into_iter())
//             .zip(party_secrets.into_iter())
//             .map(|((((i, r), ss), input), secret)| {
//                 spawn(move || {
//                     Self::run_party(
//                         self,
//                         i,
//                         n_parties,
//                         Party::without_communication_overhead(i, r, ss),
//                         input,
//                         secret,
//                     )
//                 })
//             })
//             .collect();

//         let mut all_stats = vec![];
//         let mut all_outputs = vec![];
//         for (stats, output) in handles.into_iter().map(|h| h.join().unwrap()) {
//             all_stats.push(stats);
//             all_outputs.push(output);
//         }

//         (all_stats, all_outputs)
//     }

//     /// Evaluates the protocol for a given number of parties `n_parties`, each with the input
//     /// provided by the `inputs` field.
//     fn evaluate_with_communication_overhead(
//         self,
//         n_parties: usize,
//         inputs: Vec<I>,
//         party_secrets: Vec<S>,
//         latency: Duration,
//         bytes_per_second: f64,
//     ) -> (Vec<PartyStats>, Vec<O>)
//     where
//         Self: 'static + Copy + Send,
//     {
//         assert_eq!(
//             n_parties,
//             inputs.len(),
//             "The number of parties was {} but only received {} inputs",
//             n_parties,
//             inputs.len()
//         );

//         let mut receivers = vec![];
//         let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

//         for _ in 0..n_parties {
//             let (sender, receiver) = channel();

//             receivers.push(receiver);

//             for sender_vec in senders.iter_mut() {
//                 sender_vec.push(sender.clone());
//             }
//         }

//         #[allow(clippy::needless_collect)]
//         let handles: Vec<_> = (0..n_parties)
//             .zip(receivers.into_iter())
//             .zip(senders.into_iter())
//             .zip(inputs.into_iter())
//             .zip(party_secrets.into_iter())
//             .map(|((((i, r), ss), input), secret)| {
//                 spawn(move || {
//                     Self::run_party(
//                         self,
//                         i,
//                         n_parties,
//                         Party::with_communication_overhead(i, r, ss, latency, bytes_per_second),
//                         input,
//                         secret,
//                     )
//                 })
//             })
//             .collect();

//         let mut all_stats = vec![];
//         let mut all_outputs = vec![];
//         for (stats, output) in handles.into_iter().map(|h| h.join().unwrap()) {
//             all_stats.push(stats);
//             all_outputs.push(output);
//         }

//         (all_stats, all_outputs)
//     }

//     /// Code to run one party in the protocol. The party gets a new copy of this protocol.
//     fn run_party(
//         self,
//         id: usize,
//         n_parties: usize,
//         this_party: Party,
//         input: I,
//         secret: S,
//     ) -> (PartyStats, O);
// }

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{Party, PartyStats, Protocol};

    struct ExampleParty {

    }

    impl Party for ExampleParty {
        type Input = usize;
        type Output = usize;

        fn run(&mut self, id: usize, n_parties: usize, input: Self::Input, channels: crate::comm::Channels, stats: &mut PartyStats) -> Self::Output {
            println!("Hi! I am {}/{}", id, n_parties - 1);

            let sending_timer = stats.create_timer("sending");
            for i in (id + 1)..n_parties {
                channels.send(&vec![id as u8], &i);
            }
            stats.stop_timer(sending_timer);

            for j in 0..id {
                println!(
                    "I am {}/{} and I received a message from {}",
                    id,
                    n_parties - 1,
                    channels.receive(&j).collect::<Vec<_>>()[0]
                );
            }

            id + input
        }
    }

    #[derive(Debug)]
    struct ExampleProtocol;

    impl Protocol for ExampleProtocol {
        type Input = ();
        type Output = usize;
        type Party = ;

        fn setup_parties(&self, n_parties: usize) -> Vec<Self::Party> {
            todo!()
        }

        fn generate_inputs(&self, n_parties: usize) -> Vec<Self::Input> {
            todo!()
        }
    }

    // impl Protocol<usize, usize, ()> for Example {
    //     fn run_party(
    //         self,
    //         id: usize,
    //         n_parties: usize,
    //         mut this_party: Party,
    //         input: usize,
    //         _secret: (),
    //     ) -> (PartyStats, usize) {
    //         match id {
    //             0 => this_party.set_name(String::from("Leader")),
    //             _ => this_party.set_name(format!("Assistant {}", id)),
    //         };

    //         println!("Hi! I am {}/{}", id, n_parties - 1);

    //         let sending_timer = this_party.create_timer("sending");
    //         for i in (id + 1)..n_parties {
    //             this_party.send(&vec![id as u8], &i);
    //         }
    //         this_party.stop_timer(sending_timer);

    //         for j in 0..id {
    //             println!(
    //                 "I am {}/{} and I received a message from {}",
    //                 id,
    //                 n_parties - 1,
    //                 this_party.receive(&j).collect::<Vec<_>>()[0]
    //             );
    //         }

    //         (this_party.get_stats(), id + input)
    //     }
    // }

    #[test]
    fn it_works() {
        let example = Example;
        let (stats, outputs) = example.evaluate(5, vec![10; 5], vec![(); 5]);

        println!("stats: {:?}", stats);
        assert_eq!(outputs[0], 10);
        assert_eq!(outputs[1], 11);
        assert_eq!(outputs[2], 12);
        assert_eq!(outputs[3], 13);
        assert_eq!(outputs[4], 14);
    }

    #[test]
    fn takes_longer() {
        let example = Example;

        let start = Instant::now();
        let (_, _) = example.evaluate(5, vec![10; 5], vec![(); 5]);
        let duration_1 = start.elapsed();

        let start = Instant::now();
        let (_, _) = example.evaluate_with_communication_overhead(
            5,
            vec![10; 5],
            vec![(); 5],
            Duration::from_secs(1),
            1.,
        );
        let duration_2 = start.elapsed();

        assert!(duration_2 > duration_1);
        assert!(duration_2 > Duration::from_secs(12));
    }
}
