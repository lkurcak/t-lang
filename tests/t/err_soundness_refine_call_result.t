// What a call returns is known only as its function's refinement says,
// and of the arguments it's given.
fn above(x: int) -> { r: int | r > x } with panic { x + 1 }

fn pos(n: { v: int | v > 0 }) -> int { n }

fn main() with panic {
    let a = above(-5);
    print(pos(a));
}
// error: can't prove `v > 0` for argument `n` of `pos`
