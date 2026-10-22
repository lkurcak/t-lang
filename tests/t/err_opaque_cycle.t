// The closure holds what `f` returns, which is that closure: a type that
// would hold itself.
fn f(n: i64) -> impl Fn() -> i64 with panic + Discard with panic {
    let g = f(n - 1);
    move || g() + 1
}

fn main() with panic { print(f(1)()); }
// error: the type `f` returns would contain itself
