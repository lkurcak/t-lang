// An impl for a function type is found by the type's effects, and those of
// a function that calls this one back, and says no `with`, are worked out
// with this one's: not yet known where the impl is looked for.
trait Name {
    fn name(&self) -> u32;
}

impl Name for fn(u32) -> u32 {
    fn name(&self) -> u32 { 1 }
}

fn rec(n: u32) -> u32 {
    let g = back;
    if n == 0 { 0 } else { g.name() }
}

fn back(x: u32) -> u32 {
    if x > 0 && x < 50 { rec(x - 1) } else { 0 }
}

fn main() {
    let _n = rec(3);
}
// error: `fn(u32) -> u32` doesn't implement `Name`, which method `name` is of; what its calls may do is worked out with this function's, as they call each other, so it isn't known here: say it, with `with`, where it's declared
