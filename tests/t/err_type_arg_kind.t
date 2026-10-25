// A type parameter takes a type, not a length.
struct W<T> { t: T }

fn main() {
    let _w: W<3> = W { t: 1 };
}
// error: this argument of `W` is a type, not a length
