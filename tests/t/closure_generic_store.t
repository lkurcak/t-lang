// A closure can go into generic code, which may store it. (Before 6c, this
// was rejected after monomorphisation, a few copies in.)
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn wrap<T>(x: T) -> Option<T> { Option::Some(x) }
fn twice_wrapped<T>(x: T) -> Option<T> { wrap(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let n: i64 = 1;
    match twice_wrapped(|y| y + n) {
        Option::Some(f) => out.line(&f(1))?, // expect: 2
        Option::None => {}
    }
    Result::Ok(())
}
// warning: variant `Option::None` is never constructed
