// Of a const whose value it can't tell, the logic knows its type's range,
// that of an `i64` here: if it took it to be a `usize`, as a length is,
// `NEG < 0` would contradict it, and anything would follow.
fn neg() -> { n: i64 | n < 0 } { -5 }

const NEG: { n: i64 | n < 0 } = neg();

fn positive() -> { r: i64 | r > 0 } { NEG }
// error: can't prove `r > 0` for the result of `positive`

fn main() {
    let _ = positive();
}
