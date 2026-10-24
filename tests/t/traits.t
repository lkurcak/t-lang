// Traits: a set of methods that types can implement, each in an `impl`.
use std::io::{Error, Stdout};

trait Show {
    fn show(&self, out: &mut Stdout) -> Result<(), Error>;

    /// A default, which an impl may leave out.
    fn show_twice(&self, out: &mut Stdout) -> Result<(), Error> with panic {
        self.show(out)?;
        self.show(out)
    }
}

#[derive(Discard, Copy)]
struct Point { x: i64, y: i64 }

impl Show for Point {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> {
        out.line(&self.x)?;
        out.line(&self.y)
    }
}

// Any type can implement a trait, `i64` too, and override a default.
impl Show for i64 {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> { out.line(self) }

    fn show_twice(&self, out: &mut Stdout) -> Result<(), Error> with panic {
        out.line(&(*self * 11))
    }
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

// For every `T` that's `Show`: a generic impl.
impl<T: Show> Show for Option<T> {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> {
        match self {
            Option::Some(x) => x.show(out),
            Option::None => out.line(&0i64),
        }
    }
}

// A bound: `T` must be `Show`, so values of it have `show`.
fn show_all<T: Show + Discard>(a: T, b: T, out: &mut Stdout) -> Result<(), Error> {
    a.show(out)?;
    b.show(out)
}

fn main(os: std::Os) -> Result<(), Error> with panic {
    let mut out = os.stdout;
    let p = Point { x: 1, y: 2 };
    p.show(&mut out)?;                    // expect: 1
                                          // expect: 2
    7i64.show_twice(&mut out)?;           // expect: 77
    Option::Some(3i64).show(&mut out)?;   // expect: 3
    Option::<i64>::None.show(&mut out)?;  // expect: 0
    show_all(4i64, 5, &mut out)?;         // expect: 4
                                          // expect: 5
    // Paths: the trait's method, or a type's.
    Show::show(&6i64, &mut out)?;         // expect: 6
    Point::show_twice(&p, &mut out)?;     // expect: 1
                                          // expect: 2
                                          // expect: 1
                                          // expect: 2
    let f = Option::<Point>::show;
    f(&Option::Some(p), &mut out)         // expect: 1
                                          // expect: 2
}
