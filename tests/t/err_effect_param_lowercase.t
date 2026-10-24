// Case doesn't make an effect parameter: `e` is a type parameter, as it
// isn't declared `effect e`.
fn apply<e, F: Fn() with e + Discard>(f: F) with e { f() }

fn main() { apply(|| {}) }
// error: `e` is a type parameter, not an effect; declare it as `effect e`
// warning: type parameter `e` should have an upper camel case name: `E`
