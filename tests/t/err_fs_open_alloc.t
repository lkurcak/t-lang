// Opening a file allocates its buffer, so it's `with alloc`.
use std::fs::Fs;

fn first(fs: &Fs) -> bool with {} {
    fs.open("x").is_ok() // error: alloc
}

fn main(os: std::Os) {
    let _ = first(&os.fs);
}
