// Threads share an atomic through `&`: it's `Sync`, so a `&Atomic<u32>`
// may go to another thread.
use core::sync::Atomic;
use std::thread::{JoinHandle, Threads};

fn count<'a>(threads: &Threads, n: &'a Atomic<u32>) -> JoinHandle<'a, u32> with alloc {
    threads.spawn(move || {
        let mut i = 0u32;
        while i < 100 {
            let _ = n.fetch_add(1);
            i = i + 1;
        }
        i
    })
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let n = Atomic::new(0u32);
    let a = count(&os.threads, &n);
    let b = count(&os.threads, &n);
    let c = count(&os.threads, &n);
    let done = a.join() + b.join() + c.join();
    let mut out = os.stdout;
    out.line(&done)?;     // expect: 300
    out.line(&n.load())?; // expect: 300
    Ok(())
}
