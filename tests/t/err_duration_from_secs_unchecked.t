// `from_secs` takes no more seconds than a `Duration` holds: the caller
// shows it, as with any refinement.
fn secs(n: u64) -> core::time::Duration {
    core::time::Duration::from_secs(n) // error: MAX_SECS
}

fn main() {
    let _ = secs(1);
}
