// Through a type parameter, `next` is the trait's: nothing is known of
// what it gives, even though every impl the program has says more.
struct Countdown { left: u8 }

impl Iterator for Countdown {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u8 | v < 10 }> {
        if self.left > 0 && self.left <= 10 {
            self.left = self.left - 1;
            Some(self.left)
        } else {
            None
        }
    }
}

fn first<I: Iterator<Item = u8>>(it: &mut I) -> u8 {
    let digits: [u8; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    match it.next() {
        Some(d) => digits[d as usize],
        None => 0,
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
