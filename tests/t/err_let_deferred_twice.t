// A variable declared with no value, and not `mut`, gets one value.
fn main() {
    let r: i64;
    r = 1;
    r = 2;
    print(r);
}
// error: cannot assign twice to `r`, as it isn't declared `mut`: it may have a value already
