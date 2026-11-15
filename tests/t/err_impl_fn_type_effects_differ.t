// An impl for a function type that may panic isn't one for a function
// type that can't: trait matching is by the type, with its effects exactly.
trait Name {
    fn name(&self) -> u32;
}

impl Name for fn(u32) -> u32 with panic {
    fn name(&self) -> u32 { 3 }
}

fn one(x: u32) -> u32 with {} { x }

fn main() {
    let f: fn(u32) -> u32 = one;
    let _n = f.name();
}
// error: `fn(u32) -> u32` has no method `name`; trait `Name` has one, but `fn(u32) -> u32` doesn't implement it
