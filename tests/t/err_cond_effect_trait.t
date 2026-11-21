// A trait's method can't say when it has an effect, yet.
trait Get {
    fn get(&self, i: usize) -> u8 with panic if i > 3;
}

fn main() {}
// error: a trait's method can't say when it has an effect
