// A panic in any thread stops the whole program: there's nothing to clean
// up, as nothing runs after it. What the closure may do is what `spawn`
// does: here, panic.
fn main(os: std::Os) with {alloc, panic} {
    let h = os.threads.spawn(|| {
        let v: Option<u32> = None;
        v.unwrap()
    });
    let _ = h.join();
}
// error: panicked: `unwrap` of `None`
