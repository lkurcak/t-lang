// `v` isn't exported, so what `Cell`'s `Discard` needs of `T` must be
// written down: it's part of the package's interface.

export mod cells {
    use alloc::boxed::Box;

    #[derive(Discard)]
    export struct Cell<T> { v: Box<T> }
}

fn main() {}
// error: `Cell` is exported with fields that aren't, so its `Discard` must say what it needs: `where T: Discard`
