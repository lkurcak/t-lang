// The same at the package's edge: an exported alias stands only for
// exported types.
#[derive(Discard)]
pub struct Inner { n: i64 }
export type Outer = Inner;

fn main() {}
// error: `Outer` is exported, but what it stands for names `Inner`, which isn't exported
