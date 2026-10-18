// `get`'s result lives as long as the borrow of `s`, as the trait says,
// even though the impl knows it could live as long as `'a`.
trait Get {
    fn get(&self) -> &int;
}

#[derive(Discard)]
struct S<'a> { r: &'a int }

impl<'a> Get for S<'a> {
    fn get(&self) -> &int { self.r }
}

fn main() {
    let out;
    {
        let n = Box::new(3);
        let s = S { r: &*n };
        out = s.get();
    }
    print(*out);
}
// error: `s` doesn't live long enough
