// A `Range` is only made by `a..b`: its fields are `core`'s.
fn main() {
    let r = core::ops::Range { start: 0, end: 3 };
}
// error: field `start` of `Range` is private
