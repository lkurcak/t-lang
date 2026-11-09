// Nor can a sized one stand for `str`, as ever.
fn len_of<T>(_x: &T) -> u8 { 0 }

fn main() {
    let _n = len_of("abc");
}
// error: a type parameter that isn't `unsized` can't stand for `str`
