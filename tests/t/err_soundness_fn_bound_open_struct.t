// A type whose bound says `with {}` holds only closures that do nothing,
// and a function's open bound may be given one that does more.
#[derive(Discard where F: Discard)]
struct S<F: Fn() -> i64 with {}> { f: F }

fn call<F: Fn() -> i64>(s: &S<F>) -> i64 { (s.f)() }

fn main() {}
// error: calls of `F` may have the effects `F::e`, but `S` needs calls of it to fit `fn() -> i64`
