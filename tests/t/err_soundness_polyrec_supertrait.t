// Through a supertrait's method and a default: `W<T>`'s `a` calls `b` at
// `W<&W<T>>`, whose default calls `a` there.
#[derive(Discard)]
struct W<T> { x: T }

trait A {
    fn a(&self, n: int) -> int;
}

trait B: A {
    fn b(&self, n: int) -> int { self.a(n) }
}

impl A for int {
    fn a(&self, n: int) -> int { n }
}

impl B for int {}

impl<T: B> A for W<T> {
    fn a(&self, n: int) -> int {
        if n == 0 { 0 } else { W { x: self }.b(n - 1) }
    }
}

impl<T: B> B for W<T> {}

impl<'a, T: B> A for &'a T {
    fn a(&self, n: int) -> int { (*self).a(n) }
}

impl<'a, T: B> B for &'a T {}

fn main() {
    print(W { x: 1 }.b(3));
}
// error: polymorphic recursion: this use of `B::b` with `Self` = `W<&W<T>>` leads back
