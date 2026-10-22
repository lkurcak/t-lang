trait Container {
    type Item;

    fn first(&self) -> Self::Item;
}

#[derive(Discard)]
struct Flags { on: bool }

impl Container for Flags {
    type Item = bool;

    fn first(&self) -> bool { self.on }
}

fn total<C: Container<Item = i64>>(c: &C) -> i64 { c.first() }

fn main() {
    print(total(&Flags { on: true }));
}
// error: `<Flags as Container>::Item` is `bool`, but `total` needs it to be `i64`
