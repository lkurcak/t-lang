// A panic in the core library is shown where the program called it, under
// `t` as under `t test`: here, the `unwrap`, not core's `panic`.
fn main() {
    let x: Option<u8> = Option::None;
    let _ = x.unwrap();
}
// error: panic_blame.t:5:13
