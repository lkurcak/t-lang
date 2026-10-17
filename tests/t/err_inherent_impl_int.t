impl int {
    fn double(&self) -> int { *self * 2 }
}
fn main() {}
// error: an inherent impl can only be for a struct or enum, not `int`; to give `int` methods, declare a trait and implement that
