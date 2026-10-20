// Through a blanket impl.
#[derive(Discard)]
struct W<T> { x: T }

trait Tr {
    fn deep(self, n: int) -> int with panic;
}

impl<T: Discard> Tr for T {
    fn deep(self, n: int) -> int with panic {
        if n == 0 { 0 } else { W { x: self }.deep(n - 1) }
    }
}

fn main() with panic {
    print(1.deep(3));
}
// error: polymorphic recursion: `<T as Tr>::deep` uses itself with `T` = `W<T>`
