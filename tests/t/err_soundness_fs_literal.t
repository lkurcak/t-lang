// `Fs` is intrinsic: files are reached only through the one in the `Os`
// (lemma F1 of docs/48-files.md).
fn main() {
    let fs = std::fs::Fs {}; // error: `Fs` is intrinsic: only its own functions make one or take one apart
    let _ = fs.remove("x");
}
