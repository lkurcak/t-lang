// `F: Fn(&T) -> U` takes a reference of any lifetime, as `id` does, so
// the bound's signature is `id`'s: `T` is `int`, and `U` would be `&int`
// of a call's lifetime, which means nothing outside the function type.
// `get` would then give back a reference that outlives what it points to.
// So the bound doesn't decide `U` there, and the impl doesn't apply.
trait Get {
    type Out;

    fn get(&self, x: &int) -> Self::Out;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

impl<T, U, F: Fn(&T) -> U> Get for W<F> {
    type Out = U;

    fn get(&self, x: &int) -> U { (self.f)(x) }
}

fn id(x: &int) -> &int { x }

fn main() {
    let f: fn(&int) -> &int = id;
    let w = W { f };
    let r;
    {
        let n = 5;
        r = w.get(&n);
    }
    print(*r);
}
// error: `W<fn(&int) -> &int>` doesn't implement `Get`, which method `get` is of
