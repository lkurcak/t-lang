// A legal linear expression can produce i128::MIN as a coefficient.
// Exceeding the solver's arithmetic limits must reject the proof, not panic.
fn nonnegative(
    x: { x: i64 | x * (-9223372036854775808) * (-9223372036854775808) * (-2) <= 0 },
) -> { r: i64 | r >= 0 } { x }

fn main() { let _ = nonnegative(0); }
// error: can't prove `r >= 0` for the result of `nonnegative`
