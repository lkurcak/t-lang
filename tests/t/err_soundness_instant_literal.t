// An `Instant` is one the `Clock` told: a program can't make one, so a
// function handed no `Clock` has no time to go by.
fn main() {
    let t = std::time::Instant { nanos: 0 }; // error: isn't exported
    let _ = t;
}
