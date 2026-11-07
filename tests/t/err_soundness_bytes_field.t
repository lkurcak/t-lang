// What's left of a `Bytes` is read through `rest`: its field is private.
use core::io::Bytes;

fn main() {
    let b = Bytes::new("ab".as_bytes());
    let _ = b.rest; // error: private
}
