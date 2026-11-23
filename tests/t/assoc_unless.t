// An impl's associated effect can have an atom only where a bound doesn't
// hold: `effect e = panic unless T: Low`. Of a type that's `Low`, a call of
// `go` can't panic; of one that isn't, it can; in code generic in `T`, it
// can unless a bound says `T: Low`. The impl's own method says it either
// way: `with Self::e`, or spelled out with `unless`
// (docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

/// One whose value has room for one more: a law of `value`.
trait Low: Val {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Lo { n: { v: u8 | v < 100 } }

#[derive(Discard)]
struct Any { n: u8 }

impl Val for Lo {
    fn value(&self) -> u8 { self.n }
}

impl Low for Lo {}

impl Val for Any {
    fn value(&self) -> u8 { self.n }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic unless T: Low { self.x.value() + 1 }
}

#[derive(Discard where T: Discard)]
struct V<T> { x: T }

impl<T: Val> Go for V<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with Self::e { self.x.value() + 1 }
}

/// Of a type that's `Low`: no `panic`.
fn low() -> u8 with {} { (W { x: Lo { n: 7 } }).go() + 0 * (V { x: Lo { n: 1 } }).go() }

/// Generic in one that's `Low`: none either.
fn lows<T: Low>(w: &W<T>) -> u8 with {} { w.go() }

/// Generic in any: it may, which it says by the associated effect itself.
fn any<T: Val>(w: &W<T>) -> u8 with <W<T> as Go>::e { w.go() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&low())?;                        // expect: 8
    out.line(&lows(&W { x: Lo { n: 9 } }))?;  // expect: 10
    out.line(&any(&W { x: Lo { n: 1 } }))?;   // expect: 2
    out.line(&any(&W { x: Any { n: 2 } }))?;  // expect: 3
    out.line(&(V { x: Any { n: 3 } }).go())?; // expect: 4
    Ok(())
}
