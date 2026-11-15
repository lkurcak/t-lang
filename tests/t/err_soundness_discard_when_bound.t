// An impl for empty lists only lets them be dropped where they're known to
// be empty: it doesn't make `List<Fd>` `Discard` for a bound (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn sink<T: Discard>(x: T) {}

fn main() with alloc {
    sink(List::<Fd>::new());
}
// error: `List<Fd>` doesn't implement `Discard`
