// The same for a derived `Copy`: `Q<T>` is `Copy` only if `T: Show`.
trait Show {
    fn show(&self) -> int;
}

struct Q<T> { f: fn(T) -> int }

impl<T: Show> Discard for Q<T> {}

impl<T: Show> Copy for Q<T> {}

#[derive(Discard, Copy)]
struct W<T> { q: Q<T> }

fn ignore(n: int) -> int { n }

fn dup<A: Copy>(a: A) -> int {
    let _x = a;
    let _y = a;
    2
}

fn main() {
    print(dup(W { q: Q { f: ignore } }));
}
// error: `dup` needs its `A` to implement `Copy`, and `W<int>` doesn't; that needs `int: Show`
