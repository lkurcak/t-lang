// Two modules can each have a `Point` and a `make`; each path means its
// own. A module named `main` doesn't clash with `fn main`, nor with the
// functions nested in it.
mod a {
    #[derive(Discard)]
    pub struct Point { pub v: int }

    pub fn make() -> Point { Point { v: 1 } }
}

mod b {
    #[derive(Discard)]
    pub struct Point { pub v: bool }

    pub fn make() -> Point { Point { v: true } }
}

mod main {
    pub fn helper() -> int { 10 }
}

fn main() {
    let p: a::Point = a::make();
    let q: b::Point = b::make();
    print(p.v);              // expect: 1
    print_bool(q.v);         // expect: true
    print(helper());         // expect: 20
    print(main::helper());   // expect: 10

    fn helper() -> int { 20 }
}
