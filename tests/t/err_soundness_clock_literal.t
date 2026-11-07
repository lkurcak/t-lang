// `Clock` is intrinsic: the time is read only through the one in the `Os`
// (lemma T1 of docs/49-time-entropy-terminal.md).
fn main() {
    let c = std::time::Clock {}; // error: `Clock` is intrinsic: only its own functions make one or take one apart
    let _ = c.now();
}
