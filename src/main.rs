use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{spawn, JoinHandle};

fn run(id: usize, n_parties: usize, receiver: Receiver<Vec<u8>>, senders: Vec<Sender<Vec<u8>>>) {
    println!("Hi! I am {}/{}", id, n_parties - 1);
    for i in (id+1)..n_parties {
        senders[i].send(vec![id as u8]).unwrap();
    }
    for _ in 0..id {
        let message = receiver.recv().unwrap();
        println!("I am {}/{} and I received a message from {}", id, n_parties - 1, message[0]);
    }
}

fn main() {
    let n_parties = 5;

    let mut receivers = vec![];
    let mut senders: Vec<Vec<Sender<_>>> = (0..n_parties).map(|_| vec![]).collect();

    for _ in 0..n_parties {
        let (sender, receiver) = channel();

        receivers.push(receiver);

        for j in 0..n_parties {
            senders[j].push(sender.clone());
        }
    }

    let handles: Vec<JoinHandle<_>> = (0..n_parties).zip(receivers.drain(0..n_parties)).zip(senders.drain(0..n_parties))
        .map(|((i, r), ss)| spawn(move ||
            run(i, n_parties, r, ss)))
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
