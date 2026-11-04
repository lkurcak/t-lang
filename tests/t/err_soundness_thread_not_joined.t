// A handle must be joined: dropping it would let the thread run on after
// what it borrows is gone, and nothing would wait for it.
fn main(os: std::Os) with alloc {
    let data = [1u32, 2];
    let h = os.threads.spawn(|| data[0]);
// error: `h` is never used up, and its type, `JoinHandle<u32>`, isn't `Discard`
}
