// `#[allow(unwrap_infallible)]` turns the lint off.
fn sure(x: i64) -> Result<i64, !> { Result::Ok(x) }

#[allow(unwrap_infallible)]
fn main() {
    print(sure(1).unwrap_or(0)); // expect: 1
}
