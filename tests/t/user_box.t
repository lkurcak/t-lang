// `Box` isn't a reserved name: without `use alloc::boxed::Box`, a program's
// own `Box` is an ordinary struct.
#[derive(Discard)]
struct Box { x: i64 }

fn main() {
    let b = Box { x: 3 };
    print(b.x); // expect: 3
}
