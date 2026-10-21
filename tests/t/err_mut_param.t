fn count(n: int) -> int with panic {
    n += 1;
    n
}

fn main() {}
// error: cannot assign to `n`, as `n` isn't declared `mut`
