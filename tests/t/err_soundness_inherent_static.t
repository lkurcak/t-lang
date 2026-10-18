// An inherent impl for `S<'static>` only: calling its method on a shorter
// `S` needs that to be `'static`.
#[derive(Discard)]
struct S<'a> { r: &'a int }

impl S<'static> {
    fn get(&self) -> &'static int { self.r }
}

fn main() {
    let r;
    {
        let n = Box::new(5);
        let s = S { r: &*n };
        r = s.get();
    }
    print(*r);
}
// error: `n` doesn't live long enough
