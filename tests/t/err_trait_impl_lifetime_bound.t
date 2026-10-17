// The bound would be assumed in `leak`, but nothing checks it where the
// impl is found.
struct Pair<'a, 'b> { long: &'a int, short: &'b int }

trait Leak { fn leak(self) -> Self; }

impl<'a, 'b: 'a> Leak for Pair<'a, 'b> {
    fn leak(self) -> Pair<'a, 'b> { Pair { long: self.short, short: self.short } }
}

fn main() {}
// error: `'b: 'a` says how long something lives, but an impl of a trait must hold at every lifetime, as finding the impl for a type doesn't look at lifetimes
