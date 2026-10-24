// Outside its module, a value whose type has a `discard` of its own can't
// be taken apart, even with every field public: that would skip it.
mod file {
    pub struct File { pub fd: i64 }

    impl Discard for File {
        fn discard(self) {}
    }
}

fn main() {
    let f = file::File { fd: 1 };
    let file::File { fd: _ } = f;
}
// error: can't take `File` apart here: it has a `discard` of its own, which only `file` may skip
