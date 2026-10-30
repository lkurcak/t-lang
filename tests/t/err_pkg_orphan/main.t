// An impl must be of the package's own trait or for its own type: one of
// a dependency's trait for its type would be its to write, and two
// dependents could write two.
impl shapes::Area for shapes::Square { // error: `Area` and `Square` are both `shapes`'s, so only `shapes` can implement one for the other
    fn area(&self) -> i64 { self.side * self.side }
}
fn main() {}
