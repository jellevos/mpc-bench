use std::time::{Duration, Instant};

use crate::Party;

/// Statistics pertaining to one party, such as the number of bytes sent and the durations measured.
#[derive(Debug)]
pub struct PartyStats {
    name: Option<String>,
    sent_bytes: Vec<usize>,
    measured_durations: Vec<(String, Duration)>,
}

impl PartyStats {
    pub(crate) fn new(sender_count: usize) -> Self {
        PartyStats {
            name: None,
            sent_bytes: vec![0; sender_count],
            measured_durations: vec![],
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub(crate) fn add_sent_bytes(&mut self, byte_count: usize, to_id: &usize) {
        self.sent_bytes[*to_id] += byte_count;
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

impl Party {
    /// Creates a timer with the given `name` that starts running immediately.
    pub fn create_timer(&self, name: &str) -> Timer {
        Timer::new(String::from(name))
    }

    /// Stops the `timer` and writes it measured duration to this party's statistics.
    pub fn stop_timer(&mut self, timer: Timer) {
        let (name, duration) = timer.stop();
        self.stats.write_duration(name, duration);
    }

    /// Gets the collected statistics for this party.
    pub fn get_stats(self) -> PartyStats {
        self.stats
    }
}
