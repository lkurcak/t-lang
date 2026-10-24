// A `discard` of its own is the type's module's to write, as only that
// module may take its values apart.
mod a {
    pub struct Fd { pub n: i64 }
}

impl Discard for a::Fd {
    fn discard(self) {}
}

fn main() {}
// error: `Fd`'s `discard` must be written in `a`, where it's declared
