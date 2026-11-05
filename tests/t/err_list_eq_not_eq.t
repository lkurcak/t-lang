// Lists compare only if their values do.
use alloc::list::List;

#[derive(Discard)]
struct Opaque { n: i64 }

fn main() {
    let a: List<Opaque> = List::new();
    let b: List<Opaque> = List::new();
    let _ = a == b;
}
// error: cannot compare `List<Opaque>` with `==`
