// `need` says what it's given a `T` for lives for `'a`. `caller` can't
// promise that: nothing it knows says a `T` does.
fn need<'a, T: Discard + 'a>(x: &'a i64, _t: T) -> &'a i64 { x }

fn caller<'a, T: Discard>(x: &'a i64, t: T) -> &'a i64 { need(x, t) }

fn main() {}
// error: the type parameter `T` may not live long enough: this needs it to outlive `'a`
