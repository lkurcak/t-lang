// Every lifetime in an alias is one of its parameters, or `'static`: it
// can't depend on where it's used.
type R = &i64;

fn main() {}
// error: a type alias names each lifetime in what it stands for, as in `type R<'a> = &'a i64`
