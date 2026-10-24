// Types left out are inferred from how values are used.

#[derive(Discard, Eq)]
enum Opt { None, Some(i64) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // `x`'s type is unknown until `+ 1` says it's an integer, and the call
    // says which.
    let inc = |x| x + 1;
    out.line(&inc(1i64))?;                       // expect: 2
    // Learned from a later use: the call says `x` is a bool.
    let not = |x| if x { false } else { true };
    out.line(&not(false))?;                 // expect: true
    // Through a pattern.
    let get = |o| match o { Opt::Some(n) => n, Opt::None => 0 };
    out.line(&get(Opt::Some(7)))?;               // expect: 7
    // Through `==`: `a` and `b` have the same type, which the call fixes.
    let same = |a, b| a == b;
    out.line(&same(Opt::None, Opt::None))?; // expect: true
    // Through a variable: `id`'s parameter is the same type everywhere, so
    // once it's called with an i64 it's `fn(i64) -> i64`.
    let id = |x| x;
    let y = id(3i64);
    out.line(&(y + 1))?;                           // expect: 4
    // Higher-order: `f` is a function because it's called.
    let call = |f, x| f(x);
    out.line(&call(inc, 9))?;                    // expect: 10
    Result::Ok(())
}

// A match with no arms is a `!`, which fits any type: `i64` here. (It can
// never run, so neither can this function: `_` says that's on purpose.)
#[derive(Discard)]
enum _Never {}
fn _absurd(n: _Never) -> i64 {
    match n {}
}
