// The borrow of `x` waits for the arguments, and the one after it moves
// `x`.
struct Fd { n: int }

fn two(a: &Fd, b: Fd) -> int {
    let Fd { n } = b;
    a.n + n
}

fn main() {
    let x = Fd { n: 1 };
    print(two(&x, x));
}
// error: use of moved value `x`
