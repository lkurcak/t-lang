// An impl is seen wherever its trait and type are, so what its method's
// result's refinement names must be seen everywhere: exported.
trait Size {
    fn size(&self) -> u8;
}

#[derive(Copy, Discard)]
struct P { x: u8 }

impl Size for P {
    fn size(&self) -> { v: u8 | v == self.x } { self.x }
}

fn main() {
    let _ = (P { x: 1 }).size();
}
// error: `size` is an impl's method of a trait, seen wherever the impl is, but its refinement names field `x` of `P`, which is private to the crate root
