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

fn main() with panic {
    let s = shown();
    print(s.show());                // expect: 12
    print(s.twice());               // expect: 24
    let mut i = from(5);
    let x: i64 = i.next();
    print(x);                       // expect: 6
    print(i.next() + 1);            // expect: 8
}
