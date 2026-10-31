#[derive(Copy, Discard)]
export struct Y2<P> { export x: P }

#[derive(Discard)]
struct Wrap<K> { k: K }

impl<P: q::Tr2 + Copy> q::Tr1 for Y2<P> {
    fn one<K: Discard>(self, k: K) {
        self.x.two(self, Wrap { k }) // error: polymorphic recursion: this call of `two` on a type parameter passes it `Wrap<K>` for `one`'s `K`
    }
}
