// Any bounds can follow `impl`, not just the `Fn` traits: callers know only
// what they say.
trait Show {
    fn show(&self) -> i64 with panic;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

#[derive(Discard)]
struct Two { a: i64, b: i64 }

impl Show for Two {
    fn show(&self) -> i64 with panic { self.a * 10 + self.b }
}

fn shown() -> impl Show + Discard {
    Two { a: 1, b: 2 }
}

trait Iter {
    type Item;

    fn next(&mut self) -> Self::Item with panic;
}

#[derive(Discard)]
struct Up { n: i64 }

impl Iter for Up {
    type Item = i64;

    fn next(&mut self) -> i64 with panic {
        self.n += 1;
        self.n
    }
}

fn from(n: i64) -> impl Iter<Item = i64> + Discard { Up { n } }

// A blanket impl applies to an opaque type through its bounds, and to the
// type behind it: the same impl, so the same code runs.
trait Twice {
    fn twice(&self) -> i64 with panic;
}

impl<T: Show> Twice for T {
    fn twice(&self) -> i64 with panic { self.show() * 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let s = shown();
    out.line(&s.show())?;                // expect: 12
    out.line(&s.twice())?;               // expect: 24
    let mut i = from(5);
    let x: i64 = i.next();
    out.line(&x)?;                       // expect: 6
    out.line(&(i.next() + 1))?;            // expect: 8
    Result::Ok(())
}
