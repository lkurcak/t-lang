// A barrier: each of `count` threads `wait`s, and none goes on until all
// have come. It's used again and again: here each round, every thread
// adds one, waits, checks that all have added, and waits again, so that
// none adds for the next round before all have checked. Exactly one
// thread a round is told it came last.
use core::sync::{Atomic, Barrier};
use std::thread::{JoinHandle, Threads};

const THREADS: usize = 4;
const ROUNDS: usize = 5;

#[derive(Discard, Send, Sync)]
struct Shared { barrier: Barrier, added: Atomic<usize>, wrong: Atomic<usize>, last: Atomic<usize> }

fn work(s: &Shared) {
    let mut round: usize = 0;
    while round < ROUNDS {
        let _ = s.added.fetch_add(1);
        if s.barrier.wait() {
            let _ = s.last.fetch_add(1);
        }
        if s.added.load() != THREADS * (round + 1) {
            let _ = s.wrong.fetch_add(1);
        }
        if s.barrier.wait() {
            let _ = s.last.fetch_add(1);
        }
        round = round + 1;
    }
}

fn start<'a>(threads: &Threads, s: &'a Shared) -> JoinHandle<'a, ()> with alloc {
    threads.spawn(move || work(s))
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let s = Shared {
        barrier: Barrier::new(THREADS),
        added: Atomic::new(0),
        wrong: Atomic::new(0),
        last: Atomic::new(0),
    };
    let a = start(&os.threads, &s);
    let b = start(&os.threads, &s);
    let c = start(&os.threads, &s);
    work(&s);
    a.join();
    b.join();
    c.join();
    let mut out = os.stdout;
    out.line(&s.added.load())?; // expect: 20
    out.line(&s.wrong.load())?; // expect: 0
    out.line(&s.last.load())?; // expect: 10
    Ok(())
}
