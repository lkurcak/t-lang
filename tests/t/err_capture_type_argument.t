// A closure changing a place in a tuple's element captures the element
// (K2a of docs/79-what-a-capture-splits.md): the note says so.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn tuple() {
    let mut t = (Q { x: 0, y: 10 }, 0u32);
    let mut c = || t.0.x = 5;
    t.0.y = 3; // error: the closure captures `t.0`, not just `t.0.x`, as the type of `t.0` is a type argument, which may have a refinement relating the places in it
    c();
}

fn main() {
    tuple();
}
