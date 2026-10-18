// An impl's method can't promise more than the trait's, or less.
trait Get {
    fn get(&self) -> &int;
}

#[derive(Discard)]
struct S<'a> { r: &'a int }

impl<'a> Get for S<'a> {
    fn get(&self) -> &'a int { self.r }
}

fn main() {}
// error: method `get` doesn't match its declaration in trait `Get`: it should return `&int`, not `&'a int`
