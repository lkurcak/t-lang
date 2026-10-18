// `replace` puts a value of the same type back.
fn main() {
    let mut x = 1;
    print(replace(&mut x, true));
}
// error: expected `int`, found `bool`
