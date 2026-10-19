// The closure holds what `f` returns, which is that closure: a type that
// would hold itself.
fn f(n: int) -> impl Fn() -> int + Discard {
    let g = f(n - 1);
    move || g() + 1
}

fn main() { print(f(1)()); }
// error: the type `f` returns would contain itself
