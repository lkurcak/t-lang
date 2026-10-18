// A nested `fn` is named as written in messages, not by the name it gets
// internally once it's moved to the top level.
#[derive(Discard)]
enum Option<T> { None, Some(T) }

fn main() {
    fn grow<T>(x: T) -> int { grow(Option::Some(x)) }
    print(grow(1));
}
// error: polymorphic recursion: `grow` uses itself with `T` = `Option<T>`
