use std::time::{Duration, Instant};

#[derive(Debug)]
/// Contains the aggregated statistics for multiple experiments.
pub struct AggregatedStats {
    _name: String,
    timings: Vec<Vec<Timings>>,
}

impl AggregatedStats {
    /// Constructs `AggregatedStats` with the given name for tracking statistics.
    pub fn new(name: String) -> Self {
        AggregatedStats {
            _name: name,
            timings: vec![],
        }
    }

    /// Incorporates one party's resulting statistics into this aggregate.
    pub fn incorporate_party_stats(&mut self, party_stats: Vec<Timings>) {
        self.timings.push(party_stats);
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
