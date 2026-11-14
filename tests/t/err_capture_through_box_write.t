// A closure changing a field through a box captures the box: only a
// parameter's reference is gone through.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn main() with alloc {
    let mut b = alloc::boxed::Box::new(P { x: 0, y: 10 });
    let mut c = || b.x = 5;
    b.y = 3; // error: the closure captures `b`, not just `b.x`, as `b` is a box
    c();
}
