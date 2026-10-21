// A field of a variable that can't change has the same value each time
// it's read, so what's known of it carries over (step 8.1).
#[derive(Copy, Discard)]
struct P { x: int }

fn f(p: P) -> int {
    if p.x != 0 && p.x != -1 { 10 / p.x } else { 0 }
}

fn g(r: &P) -> int {
    if r.x > 0 { 10 / r.x } else { 0 }
}

fn main() {
    print(f(P { x: 2 })); // expect: 5
    print(g(&P { x: 5 })); // expect: 2
    let p = P { x: 2 };
    print(10 / p.x); // expect: 5
}
