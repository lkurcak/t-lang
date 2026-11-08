// A field's refinement names fields of the fields before it only if
// they're seen as far as it is.
mod shapes {
    #[derive(Copy, Discard)]
    pub struct P { x: u8 }

    #[derive(Copy, Discard)]
    pub struct Q { pub p: P, pub w: { w: u8 | w > p.x } }
}

fn main() {}
// error: field `w` of `Q` is public, but its refinement names field `x` of `P`, which is private to `shapes`
