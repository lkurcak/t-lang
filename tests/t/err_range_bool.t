// A range of anything can be written, but only one of `int`s is an
// iterator.
fn main() {
    for b in false..true {
        print_bool(b);
    }
}
// error: `Range<bool>` doesn't implement `core::iter::Iterator`
