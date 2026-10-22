// `from` says it returns an `Iter`, but not what its `Item` is, so `sum`
// can't take it for one whose `Item` is `i64`: the type behind it could
// say `bool`.
trait Iter {
    type Item;

    fn next(&mut self) -> Self::Item;
}

#[derive(Discard)]
struct Up { n: i64 }

impl Iter for Up {
    type Item = i64;

    fn next(&mut self) -> i64 {
        self.n += 1;
        self.n
    }
}

fn from(n: i64) -> impl Iter + Discard { Up { n } }

fn sum<I: Iter<Item = i64> + Discard>(mut i: I) -> i64 with panic { i.next() + i.next() }

fn main() { print(sum(from(1))); }
// error: `sum` needs `<impl Iter + Discard as Iter>::Item` to be `i64`, and nothing says it is
