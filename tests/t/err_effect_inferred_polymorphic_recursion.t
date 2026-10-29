// Inferring the effects of a function that calls itself at bigger types
// ends, and then that's an error, as it is for any function.
trait Tr {
    effect e;
    fn run(&self) with Self::e;
}

#[derive(Discard)]
struct W<T> { x: T }

impl<T: Tr> Tr for W<T> {
    effect e = T::e;
    fn run(&self) with T::e { self.x.run() }
}

fn f<T: Tr + Discard>(x: T, n: u8) {
    x.run();
    if n > 0 { f(W { x }, n - 1) }
}

fn main() {}
// error: polymorphic recursion: `f` uses itself with `T` = `W<T>`
