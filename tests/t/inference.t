// Types left out are inferred from how values are used.

#[derive(Discard, Eq)]
enum Opt { None, Some(int) }

fn main() {
    // `x`'s type is unknown until `+ 1` says it's an int.
    let inc = |x| x + 1;
    print(inc(1));                          // expect: 2
    // Learned from a later use: the call says `x` is a bool.
    let not = |x| if x { false } else { true };
    print_bool(not(false));                 // expect: true
    // Through a pattern.
    let get = |o| match o { Opt::Some(n) => n, Opt::None => 0 };
    print(get(Opt::Some(7)));               // expect: 7
    // Through `==`: `a` and `b` have the same type, which the call fixes.
    let same = |a, b| a == b;
    print_bool(same(Opt::None, Opt::None)); // expect: true
    // Through a variable: `id`'s parameter is the same type everywhere, so
    // once it's called with an int it's `fn(int) -> int`.
    let id = |x| x;
    let y = id(3);
    print(y + 1);                           // expect: 4
    // Higher-order: `f` is a function because it's called.
    let call = |f, x| f(x);
    print(call(inc, 9));                    // expect: 10
}

// A match with no arms is a `!`, which fits any type: `int` here. (It can
// never run, so neither can this function: `_` says that's on purpose.)
#[derive(Discard)]
enum _Never {}
fn _absurd(n: _Never) -> int {
    match n {}
}
