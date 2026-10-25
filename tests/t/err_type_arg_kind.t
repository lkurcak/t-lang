// A type parameter takes a type, not a length.
struct W<T> { t: T }

fn main() {
    let _w: W<3> = W { t: 1 };
}
// error: `T` is a type parameter, so its argument is a type, not a length
