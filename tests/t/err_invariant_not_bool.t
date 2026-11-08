// An invariant says something: it's a `bool`.

fn f(n: usize) {
    let mut i: usize = 0;
    while i < n invariant i + 1 {
        i = i + 1;
    }
}

fn main() {}
// error: expected `bool`, found an integer
