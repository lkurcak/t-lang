// `sealed` is a keyword: the message says so where it's used as a name.
fn main() {
    let sealed = 1u8;
    let _ = sealed;
}
// error: `sealed` is a keyword, so it can't be a name
