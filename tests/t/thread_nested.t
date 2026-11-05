// A thread can start threads of its own, given a `&Threads`: `Threads` is
// `Sync`, so a reference to it may go to another thread.
use std::thread::Threads;

fn fib(threads: &Threads, n: u64) -> u64 with {alloc, panic} {
    if n < 2 {
        return n;
    }
    if n < 8 {
        return fib(threads, n - 1) + fib(threads, n - 2);
    }
    let h = threads.spawn(move || fib(threads, n - 1));
    let b = fib(threads, n - 2);
    h.join() + b
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let n = fib(&os.threads, 12);
    let mut out = os.stdout;
    out.line(&n)?; // expect: 144
    Ok(())
}
