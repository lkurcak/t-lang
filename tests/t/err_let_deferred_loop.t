// The second time round, `r` has a value from the first.
fn main() {
    let r: i64;
    let mut i = 0;
    while i < 2 {
        r = i;
        i += 1;
    }
}
// error: cannot assign twice to `r`, as it isn't declared `mut`: it may have a value already
