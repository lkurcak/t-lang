// What the thread may do, `spawn` does: a function that says it can't
// panic can't start a thread that does.
use std::thread::Threads;

fn start(threads: &Threads) -> u32 with alloc {
    threads.spawn(|| {
        let v: Option<u32> = Option::None;
        v.unwrap()
    }).join()
    // error: `spawn` may panic, but `start` doesn't say `with panic`
}

fn main(os: std::Os) with {alloc, panic} {
    let _ = start(&os.threads);
}
