fn f(n: u8) -> u8 {
    match n {
        "1" => 1,
        _ => 0,
    }
}
fn main() {
    let _ = f(1);
}
// error: expected `u8`, found `&'static str`
