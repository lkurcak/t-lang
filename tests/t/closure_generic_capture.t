// Capturing a value of a type parameter is fine whatever the type turns out
// to be, even a reference: the closure's type says it borrows `x`. (Until
// 6c this was rejected after monomorphisation, for `T` = `&i64`.)
fn call<T: Copy>(x: T) -> T {
    let g = || x;
    g()
}

fn main() {
    let n: i64 = 5;
    print(*call(&n));               // expect: 5
}
