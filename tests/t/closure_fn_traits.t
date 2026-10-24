// Closures as values of their own types, taken by generic functions through
// the `Fn` traits.

fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }
fn twice<effect e, F: FnMut() with e>(mut f: F) -> F with e {
    f();
    f();
    f
}
fn once<effect e, F: FnOnce() -> i64 with e>(f: F) -> i64 with e { f() }
fn inc(x: i64) -> i64 with panic { x + 1 }

struct B { x: i64 }

fn take(b: B) -> i64 {
    let B { x } = b;
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut stdout = os.stdout;
    // Captures by reference: `k` is only read.
    let k = 10;
    stdout.line(&apply(|x| x * k, 4))?;             // expect: 40
    // Functions implement the `Fn` traits too.
    stdout.line(&apply(inc, 1))?;                   // expect: 2
    // A closure that captures nothing is a function.
    stdout.line(&apply(|x| x - 1, 1))?;             // expect: 0
    // Changing a captured variable borrows it mutably, for as long as the
    // closure is used.
    let mut n: i64 = 0;
    let c = twice(|| n = n + 1);
    let mut c = twice(c);
    c();
    stdout.line(&n)?;                               // expect: 5
    // Moving a captured value out makes the closure `FnOnce`.
    let b = B { x: 7 };
    stdout.line(&once(|| take(b)))?;                // expect: 7
    // `Fn` closures are `FnOnce` too.
    stdout.line(&once(|| k + 1))?;                  // expect: 11
    // Lending a closure: `&F` is `Fn` when `F` is.
    let add = |x| x + k;
    stdout.line(&(apply(&add, 1) + apply(&add, 2)))?; // expect: 23
    Result::Ok(())
}
