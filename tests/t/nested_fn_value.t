// A nested function as a value shows the name it was written with.
fn main() -> fn(int) -> int {
    fn helper(x: int) -> int { x }
    print(helper(1)); // expect: 1
    helper
}
// expect: <fn helper>
