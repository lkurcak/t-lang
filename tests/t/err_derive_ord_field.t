// A derived `Ord` compares each field by its `Ord`: one without isn't.
#[derive(Discard)]
struct Opaque { n: u8 }

#[derive(Discard, Eq, Ord)]
struct W { o: Opaque }

impl Eq for Opaque {
    fn eq(&self, other: &Opaque) -> bool { self.n == other.n }
}

fn main() {}
// error: `W` can't be `Ord`: its field `o` holds a `Opaque`, which isn't
