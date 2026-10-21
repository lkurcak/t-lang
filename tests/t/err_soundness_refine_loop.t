// `d` is 1 before the loop, but not each time round it: the body changes it.
fn pos(d: { v: int | v > 0 }) -> int { d }

fn main() with panic {
    let mut d = 1;
    let mut n = 3;
    while n > 0 {
        print(pos(d));
        d = d - 1;
        n = n - 1;
    }
}
// error: can't prove `v > 0` for argument `d` of `pos`
