// A closure can go into generic code, which may store it. (Before 6c, this
// was rejected after monomorphisation, a few copies in.)
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn wrap<T>(x: T) -> Option<T> { Option::Some(x) }
fn twice_wrapped<T>(x: T) -> Option<T> { wrap(x) }

fn main() with panic {
    let n = 1;
    match twice_wrapped(|y| y + n) {
        Option::Some(f) => print(f(1)), // expect: 2
        Option::None => {}
    }
}
// warning: variant `Option::None` is never constructed
