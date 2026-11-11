// An enum's variants are seen wherever it is, so a refinement of a field
// of one names only what's seen as far.
mod m {
    #[derive(Copy, Discard)]
    pub struct S { n: u8 }

    #[derive(Copy, Discard)]
    pub enum E { A({ s: S | s.n > 3 }) }
}

fn main() {}
// error: `E` is public, but the refinement of its variant's field names field `n` of `S`, which is private to `m`
