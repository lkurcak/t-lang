// Called through the trait, as in `call`, nothing would check `by`.
trait Half {
    fn half(self, by: int) -> int;
}

impl Half for int {
    fn half(self, by: { b: int | b > 0 }) -> int { self / by }
}

fn call<T: Half>(x: T) -> int { x.half(0) }

fn main() {
    print(call(4));
}
// error: refinements can't be in a trait's methods yet
