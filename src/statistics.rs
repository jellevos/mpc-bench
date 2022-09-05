use std::{
    collections::HashMap,
    fs::File,
    time::{Duration, Instant},
};

use stats::{mean, stddev};
use tabled::{builder::Builder, Style};

#[derive(Debug)]
/// Contains the aggregated statistics for multiple repetitions of the same experiment.
pub struct AggregatedStats {
    _name: String,
    party_names: Vec<String>,
    timings: Vec<Vec<Timings>>,
}

/// The names, means and standard deviations of all parties' measured run times.
pub struct TimingSummary {
    timing_names: Vec<String>,
    party_names: Vec<String>,
    party_means: Vec<Vec<Option<f64>>>,
    party_stdevs: Vec<Vec<Option<f64>>>,
}

impl TimingSummary {
    /// Prints a pretty table of the summarized timings.
    pub fn print(&self) {
        let mut builder = Builder::default();

        // Add header
        builder.add_record(
            ["Parties".to_string()]
                .into_iter()
                .chain(self.timing_names.iter().cloned()),
        );

        // Add each party's data
        for ((means, stdevs), party_name) in self
            .party_means
            .iter()
            .zip(&self.party_stdevs)
            .zip(&self.party_names)
        {
            builder.add_record([party_name.clone()].into_iter().chain(
                means.iter().zip(stdevs).map(|data| match data {
                    (&Some(mean), &Some(stdev)) => format!("{:.3} ± {:.3} s", mean, stdev),
                    _ => "".to_string(),
                }),
            ));
        }

        let table = builder.build().with(Style::modern());

        println!("{}", table);
    }
}

impl AggregatedStats {
    /// Constructs `AggregatedStats` with the given name for tracking statistics.
    pub fn new(name: String, party_names: Vec<String>) -> Self {
        AggregatedStats {
            _name: name,
            party_names,
            timings: vec![],
        }
    }

    /// Incorporates each party's resulting statistics into this aggregate.
    pub fn incorporate_party_stats(&mut self, party_stats: Vec<Timings>) {
        self.timings.push(party_stats);
    }

    // TODO: These methods have many underlying assumptions and are not ergonomic.
    /// Outputs one party's timings to a csv named `csv_filename`.
    pub fn output_party_csv(&self, party_id: usize, csv_filename: &str) {
        // Open CSV file
        let writer = File::create(csv_filename).unwrap();
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        let headers: Vec<String> = self.timings[0][party_id]
            .measured_durations
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        csv_writer.write_record(&headers).unwrap();

        for party_timings in &self.timings {
            let durations: Vec<String> = party_timings[party_id]
                .measured_durations
                .iter()
                .map(|(_, dur)| dur.as_micros().to_string())
                .collect();
            csv_writer.write_record(&durations).unwrap();
        }

        csv_writer.flush().unwrap();
    }

    /// Summarizes the timings of all parties.
    pub fn summarize_timings(&self) -> TimingSummary {
        let mut timing_names = vec![];
        let mut party_timings_per_name: Vec<HashMap<String, Vec<f64>>> =
            (0..self.party_names.len())
                .map(|_| HashMap::new())
                .collect();

        for (party_timings, map) in self.timings.iter().zip(&mut party_timings_per_name) {
            for timing in party_timings {
                for (t, d) in &timing.measured_durations {
                    if !timing_names.contains(t) {
                        timing_names.push(t.clone());
                    }

                    map.entry(t.clone()).or_insert(vec![]).push(d.as_secs_f64());
                }
            }
        }

        println!("{:?}", party_timings_per_name);

        let party_means = (0..self.party_names.len())
            .map(|i| {
                timing_names
                    .iter()
                    .map(|t| {
                        party_timings_per_name[i]
                            .get(t)
                            .map(|durations| mean(durations.iter().cloned()))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let party_stdevs = (0..self.party_names.len())
            .map(|i| {
                timing_names
                    .iter()
                    .map(|t| {
                        party_timings_per_name[i]
                            .get(t)
                            .map(|durations| stddev(durations.iter().cloned()))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        TimingSummary {
            timing_names,
            party_names: self.party_names.clone(),
            party_means,
            party_stdevs,
        }
    }
}

/// Statistics pertaining to one party, such as the number of bytes sent and the durations measured.
#[derive(Debug)]
pub struct Timings {
    measured_durations: Vec<(String, Duration)>,
}

impl Timings {
    pub(crate) fn new() -> Self {
        Timings {
            measured_durations: vec![],
        }
    }

    pub(crate) fn write_duration(&mut self, name: String, duration: Duration) {
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

impl Timings {
    /// Creates a timer with the given `name` that starts running immediately.
    pub fn create_timer(&self, name: &str) -> Timer {
        Timer::new(String::from(name))
    }

    /// Stops the `timer` and writes it measured duration to this party's statistics.
    pub fn stop_timer(&mut self, timer: Timer) {
        let (name, duration) = timer.stop();
        self.write_duration(name, duration);
    }
}
