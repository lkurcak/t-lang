// A write through a box changes the value the box is in, as a write to a
// field does: so what a measure of it was known to be is forgotten. Every
// release before 0.76.0 kept it, as if the box were a reference, which
// only points to what changes.
mod m {
    use alloc::boxed::Box;

    #[derive(Discard)]
    pub struct Inner { pub x: u8 }

    #[derive(Discard)]
    pub struct S { pub b: Box<Inner> }

    impl S {
        #[measure]
        pub fn v(&self) -> u8 { self.b.x }

        pub fn new() -> S with alloc { S { b: Box::new(Inner { x: 1 }) } }
    }
}

fn main() with alloc {
    let mut s = m::S::new();
    let a: [u8; 2] = [7, 8];
    if s.v() == 1 {
        s.b.x = 5;
        let k: { k: u8 | k == 1 } = s.v();
        let _ = a[k as usize];
    }
}
// error: can't prove `k == 1` for `k`
