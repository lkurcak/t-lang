// A blanket impl for every `Copy` type doesn't reach one that isn't.
struct Fd { n: int }

trait Tr {
    fn f(self) -> int;
}

impl<T: Copy> Tr for T {
    fn f(self) -> int { 1 }
}

fn main() {
    print(Fd { n: 1 }.f());
}
// error: `Fd` doesn't implement `Tr`, which method `f` is of; that needs `Fd: Copy`
