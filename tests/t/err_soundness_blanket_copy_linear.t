// A blanket impl for every `Copy` type doesn't reach one that isn't.
struct Fd { n: i64 }

trait Tr {
    fn f(self) -> i64;
}

impl<T: Copy> Tr for T {
    fn f(self) -> i64 { 1 }
}

fn main() {
    print(Fd { n: 1 }.f());
}
// error: `Fd` doesn't implement `Tr`, which method `f` is of; that needs `Fd: Copy`
