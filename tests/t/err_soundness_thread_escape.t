// A handle can't outlive what its thread borrows: the thread could still be
// reading `data` after it's gone.
use std::thread::{JoinHandle, Threads};

fn start<'a>(threads: &Threads) -> JoinHandle<'a, u32> with alloc {
    let data = [1u32, 2];
    threads.spawn(|| data[1])
    // error: `data` doesn't live long enough
}

fn main(os: std::Os) with alloc {
    let _ = start(&os.threads).join();
}
