// A trait a `use` brings in can be named in a `where` clause, a
// function's or an impl's, as in `<..>`.
mod m {
    pub trait Show {
        fn show(&self) -> i64;
    }

    impl Show for i64 {
        fn show(&self) -> i64 { *self }
    }
}

use m::Show;

trait Total {
    effect e;

    fn total(&mut self) -> i64 with {Self::e, panic};
}

// `I::Item: Show` can't be an impl's bound, as `Show` isn't closed (see
// docs/29-closed-and-sealed-traits.md): the item type is a parameter of
// `W` instead, which `item` holds as a function giving one, so that `W`
// holds no `T`.
#[derive(Discard where I: Discard)]
struct W<I, T> { iter: I, item: fn() -> T }

impl<I, T> Total for W<I, T> where I: Iterator<Item = T>, T: Show + Discard {
    effect e = I::e;

    fn total(&mut self) -> i64 with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + x.show(),
                Option::None => return n,
            }
        }
    }
}

fn first<I: Iterator + Discard>(mut i: I) -> i64 with I::e where I::Item: Show + Discard {
    match i.next() {
        Option::Some(x) => x.show(),
        Option::None => 0,
    }
}

fn zero() -> i64 { 0 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(W { iter: 0..4i64, item: zero }).total())?; // expect: 6
    out.line(&first(5..7i64))?;                // expect: 5
    Result::Ok(())
}
