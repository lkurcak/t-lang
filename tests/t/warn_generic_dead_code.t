// Dead code works the same with generics: a generic function nothing
// calls is never copied, and a variant nothing builds is still dead.
#[derive(Discard)]
enum Option<T> { Some(T), None }    // warning: variant `Option::None` is never constructed
fn unused<T>(x: T) -> T { x }       // warning: function `unused` is never used
fn some<T>(x: T) -> Option<T> { Option::Some(x) }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match some(1i64) {
        Option::Some(n) => out.line(&n)?,    // expect: 1
        Option::None => {}
    }
    Result::Ok(())
}
