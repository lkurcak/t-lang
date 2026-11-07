// `Args` is intrinsic: no code makes one, so the arguments are read only
// through the one in the `Os` (lemma C1 of docs/19-std-io.md).
fn main() {
    let args = std::env::Args {}; // error: `Args` is intrinsic: only its own functions make one or take one apart
    let _ = args.len();
}
