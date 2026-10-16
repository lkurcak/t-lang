// `r` only has a value if the `if` assigns one.
fn main() {
    let r;
    let c = true;
    if c {
        r = 1;
    }
    print(r);
}
// error: use of `r`, which has no value here
