// `Token<T>` must be used up unless `T: Copy`, so a derived `Discard` of
// what holds one needs `T: Copy` too. With `T: Discard` instead, a
// `W<Thing>` could be thrown away, and the `Token<Thing>` in it with it.
struct Token<T> { f: fn(T) -> int }

impl<T: Copy> Discard for Token<T> {}

#[derive(Discard)]
struct W<T> { t: Token<T> }

#[derive(Discard)]
struct Thing { n: int }

fn ignore(t: Thing) -> int { t.n }

fn main() {
    let _w = W { t: Token { f: ignore } };
}
// error: `_w` is never used up, and its type, `W<Thing>`, isn't `Discard`
