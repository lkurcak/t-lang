// A trait a `use` brings in can be named in a `where` clause, a
// function's or an impl's, as in `<..>`.
mod m {
    pub trait Show {
        fn show(&self) -> int;
    }

    impl Show for int {
        fn show(&self) -> int { *self }
    }
}

use m::Show;

trait Total {
    effect e;

    fn total(&mut self) -> int with {Self::e, panic};
}

#[derive(Discard where I: Discard)]
struct W<I> { iter: I }

impl<I: Iterator> Total for W<I> where I::Item: Show + Discard {
    effect e = I::e;

    fn total(&mut self) -> int with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + x.show(),
                Option::None => return n,
            }
        }
    }
}

fn first<I: Iterator + Discard>(mut i: I) -> int with I::e where I::Item: Show + Discard {
    match i.next() {
        Option::Some(x) => x.show(),
        Option::None => 0,
    }
}

fn main() with panic {
    print((W { iter: 0..4 }).total()); // expect: 6
    print(first(5..7));                // expect: 5
}
