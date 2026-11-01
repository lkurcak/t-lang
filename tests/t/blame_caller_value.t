// A `#[blame_caller]` function used as a value is blamed where it became
// one: the `Option::unwrap` here, not the call inside core's `map`.
fn main() {
    let x: Option<Option<u8>> = Option::Some(Option::None);
    let _ = x.map(Option::unwrap);
}
// error: blame_caller_value.t:5:19
