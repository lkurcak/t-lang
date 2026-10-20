// A `move` closure has its own copy of `n`, but changing it still needs
// `n` to be `mut`, as it would anywhere.
fn main() with panic {
    let n = 0;
    let mut c = move || {
        n = n + 1;
        n
    };
    print(c());
}
// error: cannot assign to `n`, as `n` isn't declared `mut`
