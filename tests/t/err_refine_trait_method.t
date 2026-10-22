trait Half {
    fn half(self, by: { b: i64 | b > 0 }) -> i64;
}

fn main() {}
// error: refinements can't be in a trait's methods yet
