fn dangle(n: &i64) -> &i64 with panic {
    let x = *n + 1;
    &x
}

fn main() {}
// error: `x` doesn't live long enough: it goes away when the function returns
