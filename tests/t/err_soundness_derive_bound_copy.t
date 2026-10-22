// The same for a derived `Copy`: `Q<T>` is `Copy` only if `T: Show`.
trait Show {
    fn show(&self) -> i64;
}

struct Q<T> { f: fn(T) -> i64 }

impl<T: Show> Discard for Q<T> {}

impl<T: Show> Copy for Q<T> {}

#[derive(Discard, Copy)]
struct W<T> { q: Q<T> }

fn ignore(n: i64) -> i64 { n }

fn dup<A: Copy>(a: A) -> i64 {
    let _x = a;
    let _y = a;
    2
}

fn main() {
    print(dup(W { q: Q { f: ignore } }));
}
// error: `dup` needs its `A` to implement `Copy`, and `W<i64>` doesn't; that needs `i64: Show`
