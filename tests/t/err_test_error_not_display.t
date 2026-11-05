#[derive(Discard)]
struct E {}

fn main() {}

#[test]
fn errs() -> Result<(), E> { Err(E {}) }
// error: the error a test gives back is written out, so it must be `Display`
