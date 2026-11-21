// A value of an enum is one of the variants that can hold a value: never
// one with a field of a type with none, such as `!`, an empty enum, or a
// struct holding one. So it's known not to be one of those, and what's
// conditional on it being one doesn't happen (docs/90-conditional-effects.md).
#[derive(Discard)]
enum Void {}

// Never made: it can't be.
#[allow(dead_code)]
#[derive(Discard)]
struct Holds { v: Void, n: i64 }

fn ok_never(x: i64) -> Result<i64, !> { Ok(x) }
fn ok_void(x: i64) -> Result<i64, Void> { Ok(x) }
fn ok_holds(x: i64) -> Result<i64, Holds> { Ok(x) }
fn some_pair(x: i64) -> Result<i64, (bool, !)> { Ok(x) }

// Generic in what's in it, but not in its error.
fn first<T: Discard>(r: Result<T, !>) -> T { r.unwrap() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&ok_never(1).unwrap())?;  // expect: 1
    out.line(&ok_void(2).unwrap())?;   // expect: 2
    out.line(&ok_holds(3).unwrap())?;  // expect: 3
    out.line(&some_pair(4).unwrap())?; // expect: 4
    out.line(&first(ok_never(5)))?;    // expect: 5
    Ok(())
}
