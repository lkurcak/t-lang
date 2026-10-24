// The keyword says what's an effect parameter, so `E` here is a type
// parameter, and can't be in a `with`.
fn apply<E, F: Fn() with E>(f: F) with E { f() }

fn main() { apply(|| {}) }
// error: `E` is a type parameter, not an effect; declare it as `effect E`
