// Calling a trait's method at a bigger type is fine if the types don't keep
// growing: `W<i64>` shows an `Option<&i64>`, which shows a `&i64`, which
// shows an `i64`.
use std::io::{Error, Stdout};

trait Show {
    fn show(&self, out: &mut Stdout) -> Result<(), Error>;
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

impl<T: Show> Show for Option<T> {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> {
        match self {
            Option::Some(x) => x.show(out),
            Option::None => out.line(&0i64),
        }
    }
}

impl<'a, T: Show> Show for &'a T {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> { (**self).show(out) }
}

impl Show for i64 {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> { out.line(self) }
}

#[derive(Discard)]
struct W<T> { x: T }

impl<T: Show> Show for W<T> {
    fn show(&self, out: &mut Stdout) -> Result<(), Error> { Option::Some(&self.x).show(out) }
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    W { x: W { x: 5i64 } }.show(&mut out)?; // expect: 5
    Option::<i64>::None.show(&mut out)      // expect: 0
}
