// Which impl a trait's method runs is decided where the type has a head:
// `helper(&A { .. })` in `A`'s `one` proves `A<A<A<T>>>: Tr2` by `A`'s
// impl, and that's the `two` that `helper` calls, whatever other impls
// there are. Before step 29, a call on a type parameter was taken to go
// to every impl of the trait, `B`'s too, whose `two` calls `one` again:
// a cycle that grows, which was rejected, though monomorphising it ends
// (see docs/29-closed-and-sealed-traits.md).
trait Tr1 {
    fn one(&self) -> i64;
}

trait Tr2 {
    fn two(&self) -> i64;
}

fn call1<W: Tr1>(w: &W) -> i64 { w.one() }

fn helper<U: Tr2>(u: &U) -> i64 { u.two() }

#[derive(Copy, Discard)]
struct A<T> { t: T }

impl<T: Tr1 + Copy> Tr1 for A<T> {
    fn one(&self) -> i64 { helper(&A { t: A { t: A { t: self.t } } }) }
}

impl<T> Tr2 for A<T> {
    fn two(&self) -> i64 { 1 }
}

#[derive(Discard)]
struct B<V> { v: V }

impl<V: Tr1> Tr2 for B<V> {
    fn two(&self) -> i64 { call1(&self.v) }
}

impl Tr1 for i64 {
    fn one(&self) -> i64 { *self }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(A { t: 5i64 }).one())?; // expect: 1
    out.line(&(B { v: 7i64 }).two())?; // expect: 7
    Result::Ok(())
}
