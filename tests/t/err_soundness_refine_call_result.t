// What a call returns is known only as its function's refinement says,
// and of the arguments it's given.
fn above(x: i64) -> { r: i64 | r > x } with panic { x + 1 }

fn pos(n: { v: i64 | v > 0 }) -> i64 { n }

fn main() with panic {
    let a = above(-5);
    print(pos(a));
}
// error: can't prove `v > 0` for argument `n` of `pos`
