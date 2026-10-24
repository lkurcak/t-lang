// Generic functions, structs and enums. Each use picks its own type
// arguments, worked out from the arguments and from where the result goes.

#[derive(Discard)]
enum Option<T> { Some(T), None }
#[derive(Discard)]
struct Pair<A, B> { first: A, second: B }

fn id<T>(x: T) -> T { x }

fn flip<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    Pair { first: p.second, second: p.first }
}

// A closure has a type of its own, so `map` takes any `F` that can be
// called with a `T`. If there's no `T`, `f` isn't called, but dropped.
fn map<effect e, T, U, F: FnOnce(T) -> U with e + Discard>(o: Option<T>, f: F) -> Option<U> with e {
    match o {
        Option::Some(x) => Option::Some(f(x)),
        Option::None => Option::None,
    }
}

fn unwrap_or<T: Discard>(o: Option<T>, default: T) -> T {
    match o {
        Option::Some(x) => x,
        Option::None => default,
    }
}

// Calls a generic function inside a generic function: `id` at `T`.
fn twice<T>(f: fn(T) -> T with panic, x: T) -> T with panic { f(id(f(x))) }

// Recursion at the same type is fine.
fn repeat<T>(f: fn(T) -> T with panic, x: T, n: i64) -> T with panic {
    if n == 0 { x } else { repeat(f, f(x), n - 1) }
}

fn inc(x: i64) -> i64 with panic { x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&id(1i64))?;                                   // expect: 1
    out.line(&id(true))?;                           // expect: true
    let p = flip(Pair { first: 1i64, second: false });
    out.line(&p.first)?;                            // expect: false
    out.line(&p.second)?;                                // expect: 1
    let n: i64 = 10;
    let o = map(Option::Some(4), |x| x * n);
    out.line(&unwrap_or(o, 0))?;                         // expect: 40
    let none: Option<i64> = Option::None;
    out.line(&unwrap_or(map(none, |x| x + 1), -1))?;     // expect: -1
    out.line(&unwrap_or(map(Option::Some(3i64), |x| x > 2), false))?; // expect: true
    out.line(&twice(inc, 0))?;                           // expect: 2
    out.line(&repeat(|x| x * 2i64, 1, 10))?;                // expect: 1024
    // Generic types nest.
    let nested = Option::Some(Pair { first: Option::Some(1i64), second: 2 });
    match nested {
        Option::Some(Pair { first: Option::Some(a), second: b }) => out.line(&(a + b))?, // expect: 3
        _ => out.line(&0i64)?,
    }
    // A generic function used as a value gets one type, from how it's used.
    out.line(&twice(id, 7i64))?;                            // expect: 7
    Result::Ok(())
}
