// `replace(&mut x, x)`: `x` is moved out before it's borrowed.

use core::mem::replace;

struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

fn main() {
    let mut x = Fd { n: 1 };
    let y = replace(&mut x, x);
    print(close(y));
}
// error: use of moved value `x`
