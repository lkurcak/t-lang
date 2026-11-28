// An exported function says its effects, and a call of a value whose `Fn`
// bound says no `with` may do anything its argument does: `F::e`.
export fn apply<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 { f(x) }

fn main() {}
// error: `f` may have the effects `F::e`, but `apply` doesn't say `with F::e`
