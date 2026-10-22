// The borrow of `x` waits for the arguments, and the one after it moves
// `x`.
struct Fd { n: i64 }

fn two(a: &Fd, b: Fd) -> i64 with panic {
    let Fd { n } = b;
    a.n + n
}

fn main() with panic {
    let x = Fd { n: 1 };
    print(two(&x, x));
}
// error: use of moved value `x`
