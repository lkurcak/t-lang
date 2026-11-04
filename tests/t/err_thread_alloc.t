// Starting a thread allocates its stack, so `spawn` says `with alloc`.
use std::thread::Threads;

export fn start(threads: &Threads) -> u32 {
    threads.spawn(|| 1u32).join()
// error: `spawn` may allocate, but `start` doesn't say `with alloc`
}

fn main(os: std::Os) with alloc {
    let _ = start(&os.threads);
}
