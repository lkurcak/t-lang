// An impl for a function type with no effects isn't one for a function
// type with an effect parameter, which may stand for a set with some: so
// generic code can't use it, whatever the parameter turns out to be.
trait Name {
    fn name(&self) -> u32;
}

impl Name for fn(u32) -> u32 {
    fn name(&self) -> u32 { 3 }
}

fn call<effect e>(f: fn(u32) -> u32 with e) -> u32 { f.name() }

fn one(x: u32) -> u32 with {} { x }

fn main() {
    let _n = call(one);
}
// error: `fn(u32) -> u32 with e` has no method `name`; trait `Name` has one, but `fn(u32) -> u32 with e` doesn't implement it
