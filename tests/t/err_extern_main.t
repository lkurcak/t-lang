// C's `main` is the runtime's, which runs T's.
mod m {
    extern fn main() {}
// error: `main` is a name of the runtime's, in C
}

fn main() {}
