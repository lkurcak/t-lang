// A derived `Eq` needs `Eq` of what's in it: `Option<T>` is `Eq` if `T` is.
#[derive(Discard, Eq)]
enum Option<T> { Some(T), None }
fn is_none<T>(o: Option<T>) -> bool { o == Option::None }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&is_none(Option::Some(1)))?;  Result::Ok(()) }
// error: cannot compare `Option<T>` with `==`: that needs `T: Eq`; add a bound: `T: Eq`
