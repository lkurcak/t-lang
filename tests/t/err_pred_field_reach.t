// A field a signature's refinement names is seen at least as far as the
// function: its callers' proofs can't hang on what they can't see.
mod shapes {
    #[derive(Copy, Discard)]
    pub struct P { x: u8 }

    pub fn x_of(p: P) -> { v: u8 | v == p.x } { p.x }

    pub fn origin() -> P { P { x: 0 } }
}

fn main() {
    let _ = shapes::x_of(shapes::origin());
}
// error: `x_of` is public, but its refinement names field `x` of `P`, which is private to `shapes`
