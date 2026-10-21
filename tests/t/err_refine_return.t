// The result must have the refinement the signature promises.
fn pos(x: int) -> { r: int | r > 0 } { x }

fn main() {
    print(pos(1));
}
// error: can't prove `r > 0` for the result of `pos`
