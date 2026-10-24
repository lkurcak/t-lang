// `id`'s type, `fn(&i64) -> &i64`, takes and gives a reference of any
// lifetime a call picks. Deciding `T` and `U` by matching `Fn(T) -> U`
// against it would make them `&i64`s of a lifetime that means nothing
// outside the function type, and `call` would give back a reference that
// outlives what it points to. So the bound doesn't hold of it.
trait Call {
    type In;
    type Out;

    fn call(&self, x: Self::In) -> Self::Out;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

impl<T, U, F: Fn(T) -> U> Call for W<F> {
    type In = T;
    type Out = U;

    fn call(&self, x: T) -> U { (self.f)(x) }
}

fn id(x: &i64) -> &i64 { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f: fn(&i64) -> &i64 = id;
    let w = W { f };
    let r;
    {
        let n = 5;
        r = w.call(&n);
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: expected `<W<fn(&i64) -> &i64> as Call>::In`, found `&{integer}`
