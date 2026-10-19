// `from` says it returns an `Iter`, but not what its `Item` is, so `sum`
// can't take it for one whose `Item` is `int`: the type behind it could
// say `bool`.
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

fn from(n: int) -> impl Iter + Discard { Up { n } }

fn sum<I: Iter<Item = int> + Discard>(mut i: I) -> int { i.next() + i.next() }

fn main() { print(sum(from(1))); }
// error: `sum` needs `<impl Iter + Discard as Iter>::Item` to be `int`, and nothing says it is
