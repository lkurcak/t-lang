// A closure changing a place in a closure's parameter captures the
// parameter (K2b of docs/79-what-a-capture-splits.md): the note says so.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn h<T, F: FnOnce(T)>(x: T, f: F) { f(x) }

fn main() {
    h(Q { x: 0, y: 10 }, |mut t| {
        let mut c = || t.x = 5;
        t.y = 3; // error: the closure captures `t`, not just `t.x`, as `t` is a closure's parameter, which may be declared with a refinement where the closure is called
        c();
    });
}
