// `!`, the type with no values. Nothing of type `!` is ever made, so one
// can be used as any type: that's how a `return` fits anywhere.

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn id<T>(x: T) -> T { x }

// A function whose result is a `!` never returns.
fn forever() -> ! { loop {} }

// So this one only ever takes the first branch's way out.
fn pick(c: bool) -> i64 {
    if c { 1 } else { forever() }
}

// An `Option<!>` is never `Some`, so a `match` needs no arm for it.
fn always_none(o: Option<!>) -> i64 {
    match o {
        Option::None => 0,
    }
}

// What a `!` was wanted as, if nothing else says, is a `!`: `id::<!>`. The
// block never ends, so it's a `!` too, which fits as the `i64`.
fn returns_early() -> i64 {
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
fn _spin() -> i64 {
    let _n: ! = loop {};
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pick(true))?;                      // expect: 1
    out.line(&always_none(Option::None))?;       // expect: 0
    out.line(&returns_early())?;                 // expect: 5
    Result::Ok(())
}
