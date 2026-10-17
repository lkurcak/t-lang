struct S<T: Eq> { x: T }
fn main() {
    let _s = S { x: main };
}
// error: `S` needs its `T` to implement `Eq`, and `fn() -> ()` doesn't: functions can't be compared
