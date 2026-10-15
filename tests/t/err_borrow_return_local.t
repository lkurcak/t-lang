fn dangle(n: &int) -> &int {
    let x = *n + 1;
    &x
}

fn main() {}
// error: `x` doesn't live long enough: it goes away when the function returns
