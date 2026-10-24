// `get`'s result lives as long as the borrow of `s`, as the trait says,
// even though the impl knows it could live as long as `'a`.

use alloc::boxed::Box;

trait Get {
    fn get(&self) -> &i64;
}

#[derive(Discard)]
struct S<'a> { r: &'a i64 }

impl<'a> Get for S<'a> {
    fn get(&self) -> &i64 { self.r }
}

fn main() with alloc {
    let out;
    {
        let n = Box::new(3);
        let s = S { r: &*n };
        out = s.get();
    }
    print(*out);
}
// error: `s` doesn't live long enough
