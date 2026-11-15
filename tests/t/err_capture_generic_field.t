// A closure changing a place in a generic struct's field whose type is a
// type argument captures the field (K2a of
// docs/79-what-a-capture-splits.md), whatever the type argument is.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

fn pair() {
    let mut p = Pair { a: Q { x: 0, y: 10 }, n: 0 };
    let mut d = || p.a.x = 5;
    p.a.y = 3; // error: cannot assign to `p.a.y` while `p.a` is mutably borrowed
    d();
}

fn main() {
    pair();
}
