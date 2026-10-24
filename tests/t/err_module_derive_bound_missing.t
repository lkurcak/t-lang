// `v` is private, so what `Cell`'s `Discard` needs of `T` must be
// written down: it's part of its interface.

mod cells {
    use alloc::boxed::Box;

    #[derive(Discard)]
    pub struct Cell<T> { v: Box<T> }
}

fn main() {}
// error: `Cell` is public with private fields, so its `Discard` must say what it needs: `where T: Discard`
