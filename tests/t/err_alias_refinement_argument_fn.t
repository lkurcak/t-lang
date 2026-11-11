// An alias's parameter in a function type can't take a refinement: there
// it couldn't be written either.
type F<T> = fn(T) -> u8;

fn id(s: u8) -> u8 { s }

fn main() {
    let f: F<{ v: u8 | v < 3 }> = id;
    let _ = f(1);
}
// error: a refinement can only be
