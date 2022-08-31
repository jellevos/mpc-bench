use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::sleep,
    time::{Duration, Instant},
    vec::IntoIter,
};

use queues::{IsQueue, Queue};

/// A NetworkDescription is responsible for instantiating the networks it describes by spawning channels for each party.
pub trait NetworkDescription {
    /// Instantiates the Channels for each party.
    fn instantiate(&self, n_parties: usize) -> Vec<Channels>;
}

#[derive(Default)]
/// A full mesh network description.
pub struct FullMesh {
    latency: Duration,
    seconds_per_byte: Duration,
}

impl FullMesh {
    /// Construct a FullMesh network description without communication overhead.
    pub fn new() -> Self {
        FullMesh {
            latency: Duration::ZERO,
            seconds_per_byte: Duration::ZERO,
        }
    }

    /// Construct a FullMesh network description with the specified `latency` and bandwidth constraints (maximum `bytes_per_second`).
    pub fn new_with_overhead(latency: Duration, bytes_per_second: f64) -> Self {
        FullMesh {
            latency,
            seconds_per_byte: Duration::from_secs_f64(1. / bytes_per_second),
        }
    }
}

impl NetworkDescription for FullMesh {
    fn instantiate(&self, n_parties: usize) -> Vec<Channels> {
        let mut receivers = vec![];
        let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

        for _ in 0..n_parties {
            let (sender, receiver) = channel();

            receivers.push(receiver);

            for sender_vec in senders.iter_mut() {
                sender_vec.push(sender.clone());
            }
        }

        receivers
            .into_iter()
            .enumerate()
            .zip(senders)
            .map(|((id, r), s)| Channels::new(id, s, r, self.latency, self.seconds_per_byte))
            .collect()
    }
}

/// A message that is sent from the party with id `from_id` to another, containing a `Vec` of bytes.
pub struct Message {
    arrival_time: Instant,
    from_id: usize,
    contents: Vec<u8>,
}

/// Returns bytes with a delay, to simulate latency and bandwidth overhead
pub struct DelayedByteIterator {
    arrival_time: Instant,
    current_byte: u32,
    bytes: IntoIter<u8>,
    seconds_per_byte: Duration,
}

impl DelayedByteIterator {
    /// Creates a DelayedByteIterator for the given `bytes`, and immediately delaying for `latency`, after which each byte is returned with `seconds_per_byte` delay.
    pub fn new(bytes: Vec<u8>, arrival_time: Instant, seconds_per_byte: Duration) -> Self {
        DelayedByteIterator {
            arrival_time,
            current_byte: 0,
            bytes: bytes.into_iter(),
            seconds_per_byte,
        }
    }
}

impl Iterator for DelayedByteIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        // FIXME: Now, bandwidth runs in parallel but it should be limited
        let dur = (self.arrival_time + self.seconds_per_byte * self.current_byte) - Instant::now();
        sleep(dur);

        self.current_byte += 1;
        self.bytes.next()
    }
}

/// The communication channels for one party. These also keep track of how many bytes are sent.
pub struct Channels {
    id: usize,
    senders: Vec<Sender<Message>>,
    receiver: Receiver<Message>,
    buffer: Vec<Queue<(Instant, Vec<u8>)>>,
    sent_bytes: Vec<usize>,
    latency: Duration,
    seconds_per_byte: Duration,
}

impl Channels {
    /// Contructs a new channel with communication overhead.
    pub fn new(
        id: usize,
        senders: Vec<Sender<Message>>,
        receiver: Receiver<Message>,
        latency: Duration,
        seconds_per_byte: Duration,
    ) -> Self {
        let sender_count = senders.len();

        Channels {
            id,
            senders,
            receiver,
            buffer: (0..sender_count - 1).map(|_| Queue::new()).collect(),
            sent_bytes: vec![0; sender_count],
            latency,
            seconds_per_byte,
        }
    }

    fn add_sent_bytes(&mut self, byte_count: usize, to_id: &usize) {
        self.sent_bytes[*to_id] += byte_count;
    }

    /// Blocks until this party receives a message from the party with `from_id`. A message is a
    /// vector of bytes `Vec<u8>`. This can be achieved for example using `bincode` serialization.
    pub fn receive(&mut self, from_id: &usize) -> DelayedByteIterator {
        debug_assert_ne!(
            *from_id, self.id,
            "`from_id = {}` may not be the same as `self.id = {}`",
            from_id, self.id
        );

        let reduced_id = if *from_id < self.id {
            *from_id
        } else {
            *from_id - 1
        };

        let (arrival_time, bytes) = match self.buffer[reduced_id].size() {
            0 => loop {
                let message = self.receiver.recv().unwrap();

                if message.from_id == *from_id {
                    break (message.arrival_time, message.contents);
                }

                let message_reduced_id = if message.from_id < self.id {
                    message.from_id
                } else {
                    message.from_id - 1
                };
                self.buffer[message_reduced_id]
                    .add((message.arrival_time, message.contents))
                    .unwrap();
            },
            _ => self.buffer[reduced_id].remove().unwrap(),
        };

        
        DelayedByteIterator::new(bytes, arrival_time, self.seconds_per_byte)
    }

    /// Sends a vector of bytes to the party with `to_id` and keeps track of the number of bits sent
    /// to this party.
    pub fn send(&mut self, message: &[u8], to_id: &usize) {
        let byte_count = message.len();

        self.senders[*to_id]
            .send(Message {
                arrival_time: Instant::now() + self.latency, // FIXME: This assumes self.latency is the same as the other's latency
                from_id: self.id,
                contents: message.to_vec(),
            })
            .unwrap();

        self.add_sent_bytes(byte_count, to_id);
    }

    /// Broadcasts a message (a vector of bytes) to all parties and keeps track of the number of
    /// bits sent.
    pub fn broadcast(&mut self, message: &[u8]) {
        let byte_count = message.len();

        for sender in &self.senders {
            sender
                .send(Message {
                    arrival_time: Instant::now() + self.latency,
                    from_id: self.id,
                    contents: message.to_vec(),
                })
                .unwrap();
        }

        for i in 0..self.senders.len() {
            self.add_sent_bytes(byte_count, &i);
        }
    }
}
