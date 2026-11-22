// A function may say an effect it has only where a bound doesn't hold:
// `with panic unless T: Trait`. A call has it wherever the bound isn't
// known to hold of its type arguments, and its body is checked again with
// the bound, where nothing in it may have the effect: by what the bound
// says, a law here (docs/92-where-a-bound-holds.md).
trait Value {
    fn value(&self) -> u8;
}

/// One whose value has room for one more: a law of `value`.
trait Low: Value {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Small { n: { v: u8 | v < 100 } }

#[derive(Discard)]
struct Any { n: u8 }

impl Value for Small {
    fn value(&self) -> u8 { self.n }
}

impl Low for Small {}

impl Value for Any {
    fn value(&self) -> u8 { self.n }
}

/// One more than its value: which can't overflow of a `Low`.
trait Next: Value {
    fn next(&self) -> u8 with panic unless Self: Low { self.value() + 1 }
}

impl Next for Small {}

impl Next for Any {}

/// A function's own, of its type parameter.
fn bump<T: Value + Discard>(x: T) -> u8 with panic unless T: Low {
    let v = x.value();
    x.discard();
    v + 1
}

/// One that calls it has it where it does.
fn outer<T: Value + Discard>(x: T) -> u8 with panic unless T: Low { bump(x) }

/// With the bound, a call has none.
fn sure<T: Low + Discard>(x: T) -> u8 with {} { outer(x) }

/// Nor of a type that meets it.
fn small() -> u8 with {} { (Small { n: 7 }).next() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sure(Small { n: 5 }))?;  // expect: 6
    out.line(&small())?;               // expect: 8
    out.line(&outer(Any { n: 1 }))?;   // expect: 2
    out.line(&(Any { n: 3 }).next())?; // expect: 4
    Ok(())
}
