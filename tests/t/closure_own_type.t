// Every closure has a type of its own, even one that captures nothing: it's
// a struct with no fields, so it's `Copy` and `Discard`, and it implements
// `Fn`. Calling it, or passing it to generic code, calls its body directly.
fn apply<F: Fn(i64) -> i64 with panic + Discard>(f: F, x: i64) -> i64 with panic { f(x) }
fn twice<F: Fn(i64) -> i64 with panic + Copy + Discard>(f: F, x: i64) -> i64 with panic { apply(f, apply(f, x)) }
fn keep<F: Copy + Discard>(f: F) -> F { f }
fn adder() -> impl Fn(i64) -> i64 with panic + Discard { |x| x + 100 }

#[derive(Discard where F: Discard)]
struct Slot<F> { f: F }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let inc = |x: i64| x + 1;
    out.line(&inc(1))?;                  // expect: 2
    let copy = inc;
    out.line(&(inc(2) + copy(3)))?;     // expect: 7
    out.line(&apply(inc, 5))?;          // expect: 6
    out.line(&twice(|x| x * 3, 2))?;    // expect: 18
    out.line(&keep(inc)(9))?;           // expect: 10
    out.line(&adder()(1))?;             // expect: 101
    let s = Slot { f: |x: i64| x - 1 };
    out.line(&(s.f)(1))?;               // expect: 0
    let r = &inc;
    out.line(&apply(r, 41))?;           // expect: 42
    Result::Ok(())
}
