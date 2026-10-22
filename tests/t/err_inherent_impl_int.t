impl i64 {
    fn double(&self) -> i64 { *self * 2 }
}
fn main() {}
// error: an inherent impl can only be for a struct or enum, not `i64`; to give `i64` methods, declare a trait and implement that
