// A predicate names only fields it can see.
mod shapes {
    #[derive(Copy, Discard)]
    pub struct P { x: u8 }

    pub fn origin() -> P { P { x: 0 } }
}

fn f(p: shapes::P) -> { v: u8 | v == p.x } { 0 }

fn main() {
    let _ = f(shapes::origin());
}
// error: field `x` of `P` is private to `shapes`
