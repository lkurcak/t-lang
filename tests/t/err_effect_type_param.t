// An effect parameter is lowercase, as a type parameter is uppercase: `E`
// is a type, not an effect.
fn apply<E, F: Fn() with E>(f: F) with E { f() }

fn main() { apply(|| {}) }
// error: `E` is a type, not an effect
