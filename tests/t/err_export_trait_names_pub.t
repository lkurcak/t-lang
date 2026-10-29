// An exported trait's methods are its interface too.
#[derive(Discard)]
pub struct Hidden {}

export trait Make {
    fn make(&self) -> Hidden;
}

fn main() {}
// error: trait `Make` is exported, but it names `Hidden`, which isn't exported
