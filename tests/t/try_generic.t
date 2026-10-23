// `?` in generic code: for every `T` and `E`, linear ones too, since the
// `Ok`'s value and the `Err`'s are each moved, once.
fn again<T, E>(a: Result<T, E>) -> Result<T, E> {
    let x = a?;
    Result::Ok(x)
}

fn main() {
    let r: Result<i64, bool> = again(Result::Ok(1));
    print(r.unwrap_or(0)); // expect: 1
    let r: Result<i64, bool> = again(Result::Err(true));
    print(r.unwrap_or(0)); // expect: 0
}
