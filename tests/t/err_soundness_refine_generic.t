// Passing through a generic function forgets a refinement.
fn id<T>(x: T) -> T { x }

fn main() {
    let k: { v: int | v > 0 } = 5;
    let j: { v: int | v > 0 } = id(k);
    print(j);
}
// error: can't prove `v > 0` for `j`
