// The result must have the refinement the signature promises.
fn pos(x: i64) -> { r: i64 | r > 0 } { x }

fn main() {
    print(pos(1));
}
// error: can't prove `r > 0` for the result of `pos`
