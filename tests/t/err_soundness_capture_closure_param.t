// A closure's parameter is what it's called with, which can be declared
// with a refinement where it's called: a closure in it changing `r.x`
// captures `r`.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn call<F: FnOnce(&mut P) + Discard>(f: F, p: &mut P) {
    f(p);
}

fn main() {
    let mut p = P { x: 0, y: 10 };
    call(
        |r: &mut P| {
            let mut c = || r.x = 5;
            r.y = 3; // error: the closure captures `r`, not just `r.x`, as `r` isn't a function's parameter
            c();
        },
        &mut p,
    );
}
