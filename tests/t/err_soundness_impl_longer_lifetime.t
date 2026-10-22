// An impl's method can't promise more than the trait's, or less.
trait Get {
    fn get(&self) -> &i64;
}

#[derive(Discard)]
struct S<'a> { r: &'a i64 }

impl<'a> Get for S<'a> {
    fn get(&self) -> &'a i64 { self.r }
}

fn main() {}
// error: method `get` doesn't match its declaration in trait `Get`: it should return `&i64`, not `&'a i64`
