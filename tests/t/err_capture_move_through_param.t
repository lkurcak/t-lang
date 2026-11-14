// A `move` closure takes what it names by value, and nothing's moved out
// from behind a reference: reading `r.x`, it takes `r` (reborrowing `*r`),
// even a parameter.
#[derive(Discard)]
struct S { x: u32, y: u32 }

fn f(r: &mut S) -> u32 {
    let c = move || r.x;
    r.y = 1; // error: cannot assign to `r.y` while `*r` is mutably borrowed
    c()
}

fn main() {
    let mut s = S { x: 1, y: 2 };
    let _ = f(&mut s);
}
