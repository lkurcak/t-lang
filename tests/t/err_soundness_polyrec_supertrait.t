// Through a supertrait's method and a default: `W<T>`'s `a` calls `b` at
// `W<&W<T>>`, whose default calls `a` there.
#[derive(Discard)]
struct W<T> { x: T }

trait A {
    fn a(&self, n: i64) -> i64 with panic;
}

trait B: A {
    fn b(&self, n: i64) -> i64 with panic { self.a(n) }
}

impl A for i64 {
    fn a(&self, n: i64) -> i64 { n }
}

impl B for i64 {}

impl<T: B> A for W<T> {
    fn a(&self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { W { x: self }.b(n - 1) }
    }
}

impl<T: B> B for W<T> {}

impl<'a, T: B> A for &'a T {
    fn a(&self, n: i64) -> i64 with panic { (*self).a(n) }
}

impl<'a, T: B> B for &'a T {}

fn main() with panic {
    print(W { x: 1i64 }.b(3));
}
// error: polymorphic recursion: this use of `B::b` with `Self` = `W<&W<T>>` leads back
