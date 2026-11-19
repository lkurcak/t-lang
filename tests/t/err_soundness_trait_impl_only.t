// What only an impl promises, a call through the trait doesn't learn: it
// may be another impl's (docs/86-trait-contracts.md).
trait Get {
    fn get(&self) -> { v: u8 | v < 100 };
}

#[derive(Discard)]
struct One {}

impl Get for One {
    fn get(&self) -> { v: u8 | v == 1 } { 1 }
}

fn small<G: Get>(g: &G) -> { v: u8 | v < 2 } { g.get() }

fn main() { let _ = small(&One {}); }
// error: can't prove `v < 2` for the result of `small`
