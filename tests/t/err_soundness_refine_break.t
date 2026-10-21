// A `break` leaves the loop with its condition still true.
fn pos(n: { v: int | v > 0 }) -> int { n }

fn f(stop: bool) -> int with panic {
    let mut x = 0;
    while x < 10 {
        if stop {
            break;
        }
        x = x + 1;
    }
    pos(x - 9)
}

fn main() with panic {
    print(f(true));
}
// error: can't prove `v > 0` for argument `n` of `pos`
