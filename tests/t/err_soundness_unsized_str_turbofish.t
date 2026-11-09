// `str` as a type argument, but for a parameter with a size.
fn len_of<T>(_x: &T) -> u8 { 0 }

fn main() {
    let _n = len_of::<str>("abc");
}
// error: `str` can only be behind a shared reference, as in `&str`
