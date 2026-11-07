// `Entropy` is intrinsic: unguessable bytes come only through the one in
// the `Os` (lemma T1 of docs/49-time-entropy-terminal.md).
fn main() {
    let e = std::entropy::Entropy {}; // error: `Entropy` is intrinsic: only its own functions make one or take one apart
    let _ = e.u64();
}
