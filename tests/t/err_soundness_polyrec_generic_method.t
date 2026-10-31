// The loop of tests/t/err_pkg_generic_method_growth, in one program:
// `Y2<X>`'s `one::<K>` calls `X`'s `two::<Y2<X>, Wrap<K>>`, which calls
// `one::<Wrap<K>>`, and monomorphising that never ends.
trait Tr1 {
    fn one<K: Discard>(self, k: K);
}

trait Tr2 {
    fn two<N: Tr1, K: Discard>(self, n: N, k: K);
}

#[derive(Copy, Discard)]
struct Y2<P> { x: P }

#[derive(Discard)]
struct Wrap<K> { k: K }

impl<P: Tr2 + Copy> Tr1 for Y2<P> {
    fn one<K: Discard>(self, k: K) { self.x.two(self, Wrap { k }) }
}

#[derive(Copy, Discard)]
struct X {}

impl Tr2 for X {
    fn two<N: Tr1, K: Discard>(self, n: N, k: K) { n.one(k) }
}

fn main() {
    (Y2 { x: X {} }).one(0u8);
}
// error: polymorphic recursion: this call of `two` on a type parameter passes it `Wrap<K>` for `one`'s `K`
