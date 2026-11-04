// The right to start threads can't be made: only `main` is given it.
use std::thread::Threads;

fn main() with alloc {
    let t = Threads {};
    let _ = t.spawn(|| 1u32).join();
}
// error: `Threads` is intrinsic: only its own functions make one or take one apart
