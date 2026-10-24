// A program's own `Box` is no box: there's nothing to dereference.
#[derive(Discard)]
struct Box<T> { v: T }

fn main() {
    let b = Box { v: 1 };
    let _x: i64 = *b;
}
// error: can't use `*` on `Box<_>`: only on a reference or a box
