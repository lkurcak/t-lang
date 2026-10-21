// What's known of a field isn't tracked yet (step 8.1), so this can't be
// proved; it would be wrong to assume a field read twice is the same.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(p: P) -> int {
    if p.x != 0 && p.x != -1 { 10 / p.x } else { 0 }
}

fn main() with panic {
    print(f(P { x: 2 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
