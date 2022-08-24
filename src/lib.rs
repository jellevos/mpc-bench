#![doc = include_str!("../README.md")]
#![warn(missing_docs, unused_imports)]

use comm::{Channels, NetworkDescription};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::fmt::Debug;

use statistics::{AggregatedStats, Timings};

/// Communication module, allows parties to send and receive messages.
pub mod comm;

/// Statistics module, allows parties to track timings and bandwidth costs.
pub mod statistics;

/// A `Party` that takes part in a protocol. The party will receive a unique `id` when it is running the protocol, as well as
/// communication channels to and from all the other parties. A party keeps track of its own stats.
pub trait Party {
    /// The input type of this party. It must be the same for all parties in a given protocol (but it could be e.g. an enum or Option).
    type Input: Send;
    /// The output type of this party. It must be the same for all parties in a given protocol (but it could be e.g. an enum or Option)
    type Output: Debug + Send;

    /// Gets the name of this party. By default, this is 'Party {id}'.
    fn get_name(&self, id: usize) -> String {
        format!("Party {}", id)
    }

    /// Runs the code for this party in the given protocol. The `id` starts from 0.
    fn run(
        &mut self,
        id: usize,
        n_parties: usize,
        input: &Self::Input,
        channels: &mut Channels,
        timings: &mut Timings,
    ) -> Self::Output;
}

/// MPC protocols are described by the `Protocol` trait for a given `Party` type that can be sent accross threads. An implementation should hold the protocol-specific parameters.
pub trait Protocol
where
    Self: Debug,
{
    /// The type of the parties participating in the Protocol.
    type Party: Party + Send;

    /// Sets up `n_parties` according to this parameterization of the Protocol.
    fn setup_parties(&self, n_parties: usize) -> Vec<Self::Party>;

    /// Generates each party's potentially random input for this parameterization of the Protocol.
    fn generate_inputs(&self, n_parties: usize) -> Vec<<Self::Party as Party>::Input>;

    /// Validates the outputs of one run of the Protocol. If false, `evaluate` will print a warning.
    fn validate_outputs(
        &self,
        _inputs: &[<Self::Party as Party>::Input],
        _outputs: &[<Self::Party as Party>::Output],
    ) -> bool {
        true
    }

    /// Evaluates multiple `repetitions` of the protocol with this parameterization of the Protocol.
    fn evaluate<N: NetworkDescription>(
        &self,
        experiment_name: String,
        n_parties: usize,
        network_description: &N,
        repetitions: usize,
    ) -> AggregatedStats {
        let mut parties = self.setup_parties(n_parties);
        debug_assert_eq!(parties.len(), n_parties);

        let mut stats = AggregatedStats::new(
            experiment_name,
            parties
                .iter()
                .enumerate()
                .map(|(id, party)| party.get_name(id))
                .collect(),
        );

        for _ in 0..repetitions {
            let mut inputs = self.generate_inputs(n_parties);
            debug_assert_eq!(inputs.len(), n_parties);

            let mut channels = network_description.instantiate(n_parties);
            debug_assert_eq!(channels.len(), n_parties);

            let mut party_timings: Vec<Timings> = (0..n_parties).map(|_| Timings::new()).collect();

            let outputs: Vec<_> = parties
                .par_iter_mut()
                .enumerate()
                .zip(inputs.par_iter_mut())
                .zip(channels.par_iter_mut())
                .zip(party_timings.par_iter_mut())
                .map(|((((id, party), input), channel), s)| {
                    party.run(id, n_parties, input, channel, s)
                })
                .collect();

            if !self.validate_outputs(&inputs, &outputs) {
                println!(
                    "The outputs are invalid:\n{:?} ...for these parameters:\n{:?}",
                    outputs, self
                );
                // TODO: Mark invalid in stats
            }

            // TODO: Incorporate communication costs
            println!("{:?}", party_timings);
            stats.incorporate_party_stats(party_timings);
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{
        comm::{Channels, FullMesh},
        Party, Protocol, Timings,
    };

    struct ExampleParty;

    impl Party for ExampleParty {
        type Input = usize;
        type Output = usize;

        fn run(
            &mut self,
            id: usize,
            n_parties: usize,
            input: &Self::Input,
            channels: &mut Channels,
            stats: &mut Timings,
        ) -> Self::Output {
            println!("Hi! I am {}/{}", id, n_parties - 1);

            let sending_timer = stats.create_timer("Sending");
            for i in (id + 1)..n_parties {
                channels.send(&vec![id as u8], &i);
            }
            stats.stop_timer(sending_timer);

            let receiving_timer = stats.create_timer("Receiving");
            for j in 0..id {
                println!(
                    "I am {}/{} and I received a message from {}",
                    id,
                    n_parties - 1,
                    channels.receive(&j).collect::<Vec<_>>()[0]
                );
            }
            stats.stop_timer(receiving_timer);

            id + input
        }
    }

    #[derive(Debug)]
    struct ExampleProtocol;

    impl Protocol for ExampleProtocol {
        type Party = ExampleParty;

        fn setup_parties(&self, n_parties: usize) -> Vec<Self::Party> {
            (0..n_parties).map(|_| ExampleParty).collect()
        }

        fn generate_inputs(&self, n_parties: usize) -> Vec<usize> {
            (0..n_parties).map(|_| 10).collect()
        }

        fn validate_outputs(
            &self,
            inputs: &[<Self::Party as Party>::Input],
            outputs: &[<Self::Party as Party>::Output],
        ) -> bool {
            for i in 0..outputs.len() {
                if outputs[i] != (inputs[i] + i) {
                    return false;
                }
            }

            true
        }
    }

    #[test]
    fn it_works() {
        let example = ExampleProtocol;
        let network = FullMesh::new();
        let stats = example.evaluate("Experiment".to_string(), 5, &network, 1);

        println!("stats: {:?}", stats);
        // FIXME: All rows are aggregated instead of party-by-party
        stats.summarize_timings().print();
    }

    #[test]
    fn takes_longer() {
        let example = ExampleProtocol;

        let start = Instant::now();
        let network = FullMesh::new();
        let _ = example.evaluate("Experiment".to_string(), 5, &network, 1);
        let duration_1 = start.elapsed();

        let start = Instant::now();
        let network = FullMesh::new_with_overhead(Duration::from_secs(1), 1.);
        let stats = example.evaluate("Experiment (w/ overhead)".to_string(), 5, &network, 1);
        let duration_2 = start.elapsed();

        assert!(duration_2 > duration_1);
        assert!(duration_2 > Duration::from_secs(12));

        stats.summarize_timings().print();
    }
}
