// A nested function as a value shows the name it was written with.
fn main() -> fn(i64) -> i64 {
    fn helper(x: i64) -> i64 { x }
    print(helper(1)); // expect: 1
    helper
}
// expect: <fn helper>
