// The pieces `split` gives are of the string it splits, so they can't
// outlive it.
use alloc::string::String;

fn main() with alloc {
    let first: &str;
    {
        let s = String::from("a,b");
        first = s.as_str().split(',').next().unwrap_or("");
    }
    let _ = first.len();
}
// error: `s` doesn't live long enough
