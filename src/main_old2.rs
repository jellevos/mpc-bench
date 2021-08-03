use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender, channel};

fn run(id: u32, n_parties: u32, receiver: &Receiver<Vec<u8>>, senders: Vec<Sender<Vec<u8>>>) {
    println!("Hi I am {}!", id);


}

struct Party {
    id: u32,
    sender: Sender<Vec<u8>>,
    receiver: Receiver<Vec<u8>>,
}

impl Party {
    fn new(id: u32) -> Self {
        let (sender, receiver) = channel();

        Party {
            id,
            sender,
            receiver,
        }
    }
}

fn main() {
    let n = 5;

    let parties: Vec<Party> = (0..n).map(|id| Party::new(id)).collect();

    let join_handles: Vec<JoinHandle<_>> = parties.iter().map(|party| {
        let senders: Vec<Sender<Vec<u8>>> = parties.iter().map(|p| p.sender.clone()).collect();
        thread::spawn(move || run(party.id, n, &party.receiver, senders))
    }).collect();

    for join_handle in join_handles {
        join_handle.join();
    }
}
