trait A: B {}
trait B: A {}
fn main() {}
// error: `B` can't be a supertrait of `A`
