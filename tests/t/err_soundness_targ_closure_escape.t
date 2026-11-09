// A closure given to a function that gives it back can't count on what the
// function's other arguments say of its parameter: whoever gets it back
// can call it with anything.
fn keep<A, F: Fn(A) -> i64 + Discard>(f: F, a: A) -> (F, A) { (f, a) }

fn main() {
    let (f, _) = keep(|x: i64| x * 2, 3);
    let _ = f(4_611_686_018_427_387_904);
}
// error: this closure may panic
