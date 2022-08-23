use std::{thread::sleep, time::Duration, vec::IntoIter};

use crate::Party;

use queues::IsQueue;

/// A message that is sent from the party with id `from_id` to another, containing a `Vec` of bytes.
pub(crate) struct Message {
    from_id: usize,
    contents: Vec<u8>,
}

/// Returns bytes with a delay, to simulate latency and bandwidth overhead
pub struct DelayedByteIterator {
    bytes: IntoIter<u8>,
    seconds_per_byte: Duration,
}

impl DelayedByteIterator {
    /// Creates a DelayedByteIterator for the given `bytes`, and immediately delaying for `latency`, after which each byte is returned with `seconds_per_byte` delay.
    pub fn new(bytes: Vec<u8>, latency: Duration, seconds_per_byte: Duration) -> Self {
        sleep(latency);
        DelayedByteIterator {
            bytes: bytes.into_iter(),
            seconds_per_byte,
        }
    }
}

impl Iterator for DelayedByteIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        sleep(self.seconds_per_byte);
        self.bytes.next()
    }
}

impl Party {
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

        let bytes = match self.buffer[reduced_id].size() {
            0 => loop {
                let message = self.receiver.recv().unwrap();

                if message.from_id == *from_id {
                    break message.contents;
                }

                let message_reduced_id = if message.from_id < self.id {
                    message.from_id
                } else {
                    message.from_id - 1
                };
                self.buffer[message_reduced_id]
                    .add(message.contents)
                    .unwrap();
            },
            _ => self.buffer[reduced_id].remove().unwrap(),
        };

        DelayedByteIterator::new(bytes, self.latency, self.seconds_per_byte)
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
}
