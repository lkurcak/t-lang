// `!`, the type with no values. Nothing of type `!` is ever made, so one
// can be used as any type: that's how a `return` fits anywhere.

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn id<T>(x: T) -> T { x }

// A function whose result is a `!` never returns.
fn forever() -> ! { loop {} }

// So this one only ever takes the first branch's way out.
fn pick(c: bool) -> int {
    if c { 1 } else { forever() }
}

// An `Option<!>` is never `Some`, so a `match` needs no arm for it.
fn always_none(o: Option<!>) -> int {
    match o {
        Option::None => 0,
    }
}

// What a `!` was wanted as, if nothing else says, is a `!`: `id::<!>`. The
// block never ends, so it's a `!` too, which fits as the `int`.
fn returns_early() -> int {
    let _x = id(return 5);
}

// A `match` with no arms is a `!`, and so is a `loop` that nothing breaks
// out of, as the annotations say. Neither function can be called, or would
// return if it were: `_` says that's on purpose.
#[derive(Discard)]
enum _Void {}
fn _absurd(v: _Void) -> bool {
    let _b: ! = match v {};
}
fn _spin() -> int {
    let _n: ! = loop {};
}

fn main() {
    print(pick(true));                      // expect: 1
    print(always_none(Option::None));       // expect: 0
    print(returns_early());                 // expect: 5
}
