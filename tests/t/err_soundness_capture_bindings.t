// What a `for`, a `match` or a `let` taking a value apart binds declares
// what its value's type has, as a `let` does: a closure changing a place
// in a tuple's element there captures the element (K2a of
// docs/79-what-a-capture-splits.md). So does a copy of a variable declared
// with one.
use alloc::list::List;

#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn some() -> Option<({ v: Q | v.x <= v.y }, u32)> { Some((Q { x: 0, y: 10 }, 0)) }

fn pair() -> (({ v: Q | v.x <= v.y }, u32), u8) { ((Q { x: 0, y: 10 }, 0), 0) }

fn main() with alloc {
    let mut l: List<({ v: Q | v.x <= v.y }, u32)> = List::new();
    l.push((Q { x: 0, y: 10 }, 0));
    for mut t in l {
        let mut c = || t.0.x = 50; // error: can't prove `v.x <= v.y` for `t.0`, once `t.0.x` is written
        c();
        need(&t.0);
    }
    match some() {
        Some(mut t) => {
            let mut c = || t.0.x = 50; // error: can't prove `v.x <= v.y` for `t.0`, once `t.0.x` is written
            c();
            need(&t.0);
        },
        None => {},
    }
    let (mut t, _) = pair();
    let mut c = || t.0.x = 50; // error: can't prove `v.x <= v.y` for `t.0`, once `t.0.x` is written
    c();
    need(&t.0);
    let a: ({ v: Q | v.x <= v.y }, u32) = (Q { x: 0, y: 10 }, 0);
    let mut u = a;
    let mut d = || u.0.x = 50; // error: can't prove `v.x <= v.y` for `u.0`, once `u.0.x` is written
    d();
    need(&u.0);
}
