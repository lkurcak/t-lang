// `is_some` of a `None` is false, so a `None` can't be given where a
// `Some` is promised.
fn some() -> { r: Option<u8> | r.is_some() } { None }

fn main() {
    let _ = some();
}
// error: can't prove `r.is_some()` for the result of `some`
