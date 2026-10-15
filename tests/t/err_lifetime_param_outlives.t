// `need` says what it's given a `T` for lives for `'a`. `caller` can't
// promise that: nothing it knows says a `T` does.
fn need<'a, T: 'a>(x: &'a int, _t: T) -> &'a int { x }

fn caller<'a, T>(x: &'a int, t: T) -> &'a int { need(x, t) }

fn main() {}
// error: the type parameter `T` may not live long enough: this needs it to outlive `'a`
