// `replace(&mut x, x)`: `x` is moved out before it's borrowed.

use core::mem::replace;

struct Fd { n: int }

fn close(f: Fd) -> int {
    let Fd { n } = f;
    n
}

fn main() {
    let mut x = Fd { n: 1 };
    let y = replace(&mut x, x);
    print(close(y));
}
// error: use of moved value `x`
