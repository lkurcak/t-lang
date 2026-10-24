// Closures: anonymous functions that can use the locals around them.

#[derive(Discard)]
struct P { x: i64, y: i64 }

// A closure that captures has a type of its own, so a function that takes
// one is generic over it.
fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }
fn compose<effect e, F: Fn(i64) -> i64 with e + Discard, G: Fn(i64) -> i64 with e + Discard>(
    f: F,
    g: G,
    x: i64,
) -> i64 with e {
    // A closure can capture a parameter.
    apply(|y| g(f(y)), x)
}
fn on_point<effect e, F: Fn(P) -> i64 with e + Discard>(f: F) -> i64 with e { f(P { x: 3, y: 4 }) }
fn inc(x: i64) -> i64 with panic { x + 1 }
// Closures that capture nothing can be returned: they're plain functions.
fn pick(double: bool) -> fn(i64) -> i64 with panic {
    if double { |x| x * 2 } else { inc }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let k = 10;
    out.line(&apply(|x| x * k, 4))?;             // expect: 40
    let five = 5;
    let add5 = |x| x + five;
    out.line(&add5(1))?;                         // expect: 6
    out.line(&compose(add5, |x| x + 100, 0))?;   // expect: 105
    // Functions work where closures do.
    out.line(&compose(inc, |x| x * 2, 3))?;      // expect: 8
    out.line(&pick(true)(pick(false)(4)))?;      // expect: 10
    // `p`'s type comes from `on_point`'s bound, so `p.x` works.
    out.line(&on_point(|p| p.x * p.y))?;         // expect: 12
    // No parameters, annotations, a block body.
    let one = || 1;
    let both = |a: i64, b| -> i64 { let s = a + b; s * s };
    out.line(&(one() + both(1, 2)))?;              // expect: 10
    // A closure that borrows `n` sees the `n` it was made with, as the
    // `let` after makes a new one.
    let n: i64 = 1;
    let get = || n;
    let n = 2;
    out.line(&(get() + n))?;                       // expect: 3
    // Closures in data, if they capture nothing.
    let fs = Fs { a: |x| x - 1, b: pick(true) };
    out.line(&(fs.a)((fs.b)(3)))?;               // expect: 5
    out.line(&apply(fs.a, 1))?;                  // expect: 0
    // Immediately called.
    out.line(&(|x| x + 1i64)(41))?;                 // expect: 42
    // Calling an argument with unknown type makes it a function.
    let twice = |f, x| f(f(x));
    out.line(&twice(inc, 0))?;                   // expect: 2
    Result::Ok(())
}

#[derive(Discard)]
struct Fs { a: fn(i64) -> i64 with panic, b: fn(i64) -> i64 with panic }
