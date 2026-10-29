// An alias in a function's body can't use the function's type parameters,
// as a local struct can't.
fn f<T>(x: T) -> T {
    type Same = T;
    let y: Same = x;
    y
}

fn main() {}
// error: can't use type parameter `T` of the enclosing function in a local `type`; declare it on this one too, as in `type Same<T>`
