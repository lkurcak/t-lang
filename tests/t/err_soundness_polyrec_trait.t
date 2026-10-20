// Polymorphic recursion through impls of a trait: `W<T>`'s `go` uses `go`
// at `W<&W<T>>`.
trait Tr {
    fn go(&self, n: int) -> int with panic;
}

#[derive(Discard)]
struct W<T> { x: T }

impl Tr for int {
    fn go(&self, n: int) -> int with panic {
        if n == 0 { *self } else { W { x: *self }.go(n - 1) }
    }
}

impl<T: Tr> Tr for W<T> {
    fn go(&self, n: int) -> int with panic {
        if n == 0 { 0 } else { W { x: self }.go(n - 1) }
    }
}

impl<'a, T: Tr> Tr for &'a T {
    fn go(&self, n: int) -> int with panic { (*self).go(n) }
}

fn main() with panic {
    print(1.go(3));
}
// error: polymorphic recursion: `<W<T> as Tr>::go` uses itself with `T` = `&W<T>`
