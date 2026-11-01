// A panic in core's `#[blame_caller]` functions is shown where the program
// called them, under `t` as under `t test`: here, the `unwrap`, not core's
// `panic`.
fn main() {
    let x: Option<u8> = Option::None;
    let _ = x.unwrap();
}
// error: panic_blame.t:6:13
