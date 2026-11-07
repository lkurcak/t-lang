// A `ReadFile` is intrinsic: only `Fs::open` makes one, so each is a file
// that was opened, and is closed once (lemma F1 of docs/48-files.md).
fn main() {
    let f = std::fs::ReadFile {}; // error: `ReadFile` is intrinsic: only its own functions make one or take one apart
    f.discard();
}
