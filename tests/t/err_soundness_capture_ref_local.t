// A reference that isn't a parameter can point to a variable declared
// with a refinement, which the reference says: a closure changing `r.x`
// captures `r`.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn main() {
    let mut p: { v: P | v.x < v.y } = P { x: 0, y: 10 };
    let r = &mut p;
    let mut c = || r.x = 5;
    r.y = 3; // error: the closure captures `r`, not just `r.x`, as `r` isn't a function's parameter
    c();
}
