// A range is of integers.
fn main() {
    let _r = 'a'..'z';
}
// error: `Range<T>::new` needs its `T` to implement `Int`, and `char` doesn't
