// Passing through a generic function forgets a refinement when the
// function could make a new value of its type parameter: a `T: Int` can be
// any integer it likes. (An `id<T>` with no bound keeps it: the free
// theorem of docs/55-free-theorems.md.)
fn id<T: Int>(x: T) -> T { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k: { v: i64 | v > 0 } = 5;
    let j: { v: i64 | v > 0 } = id(k);
    out.line(&j)?;
    Ok(())
}
// error: can't prove `v > 0` for `j`
