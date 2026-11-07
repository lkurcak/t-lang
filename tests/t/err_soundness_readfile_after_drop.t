// A file dropped is closed: it can't be read after (lemma F2 of
// docs/48-files.md).
use alloc::string::String;

fn main(os: std::Os) with alloc {
    if let Ok(f) = os.fs.open("x") {
        let mut f = f;
        f.discard();
        let mut s = String::new();
        let _ = f.read_line(&mut s); // error: moved
        s.discard();
    }
}
