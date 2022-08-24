use std::time::{Duration, Instant};

#[derive(Debug)]
/// Contains the aggregated statistics for multiple experiments.
pub struct AggregatedStats {
    _name: String,
    stats: Vec<PartyStats>,
}

impl AggregatedStats {
    /// Constructs `AggregatedStats` with the given name for tracking statistics.
    pub fn new(name: String) -> Self {
        AggregatedStats {
            _name: name,
            stats: vec![],
        }
    }

    /// Incorporates one party's resulting statistics into this aggregate.
    pub fn incorporate_party_stats(&mut self, party_stats: PartyStats) {
        self.stats.push(party_stats);
    }
}

/// Statistics pertaining to one party, such as the number of bytes sent and the durations measured.
#[derive(Debug)]
pub struct PartyStats {
    measured_durations: Vec<(String, Duration)>,
}

impl PartyStats {
    pub(crate) fn new() -> Self {
        PartyStats {
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

impl PartyStats {
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
