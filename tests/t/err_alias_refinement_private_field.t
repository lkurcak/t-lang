// A public alias's refinement names only what's seen as far as it is.
mod m {
    #[derive(Copy, Discard)]
    pub struct S { n: u8 }

    pub type Big = { s: S | s.n > 3 };
}

fn main() {}
// error: `Big` is public, but its refinement names field `n` of `S`, which is private to `m`
