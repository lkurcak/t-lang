// What one function's bounds say isn't known in the next.
struct Fd { n: i64 }

fn a<T: Discard>(_x: T) {}

fn b<T>(_x: T) {}

fn main() {
    a(1);
    b(Fd { n: 1 });
}
// error: `_x` is never used up, and its type, `T`, isn't `Discard`; to let it be thrown away, add a bound: `T: Discard`
