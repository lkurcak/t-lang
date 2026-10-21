trait Half {
    fn half(self, by: { b: int | b > 0 }) -> int;
}

fn main() {}
// error: refinements can't be in a trait's methods yet
