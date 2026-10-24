// A nested `fn` is named as written in messages, not by the name it gets
// internally once it's moved to the top level.
#[derive(Discard)]
enum Option<T> { None, Some(T) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    fn grow<T>(x: T) -> i64 { grow(Option::Some(x)) }
    out.line(&grow(1i64))?;
    Result::Ok(())
}
// error: polymorphic recursion: `grow` uses itself with `T` = `Option<T>`
