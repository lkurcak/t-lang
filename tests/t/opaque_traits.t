// Any bounds can follow `impl`, not just the `Fn` traits: callers know only
// what they say.
trait Show {
    fn show(&self) -> int with panic;
}

impl Show for int {
    fn show(&self) -> int { *self }
}

#[derive(Discard)]
struct Two { a: int, b: int }

impl Show for Two {
    fn show(&self) -> int with panic { self.a * 10 + self.b }
}

fn shown() -> impl Show + Discard {
    Two { a: 1, b: 2 }
}

trait Iter {
    type Item;

    fn next(&mut self) -> Self::Item;
}

#[derive(Discard)]
struct Up { n: int }

impl Iter for Up {
    type Item = int;

    fn next(&mut self) -> int {
        self.n += 1;
        self.n
    }
}

fn from(n: int) -> impl Iter<Item = int> + Discard { Up { n } }

// A blanket impl applies to an opaque type through its bounds, and to the
// type behind it: the same impl, so the same code runs.
trait Twice {
    fn twice(&self) -> int with panic;
}

impl<T: Show> Twice for T {
    fn twice(&self) -> int with panic { self.show() * 2 }
}

fn main() with panic {
    let s = shown();
    print(s.show());                // expect: 12
    print(s.twice());               // expect: 24
    let mut i = from(5);
    let x: int = i.next();
    print(x);                       // expect: 6
    print(i.next() + 1);            // expect: 8
}
