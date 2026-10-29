// An exported field's type must be exported too.
#[derive(Discard)]
pub struct In {}

#[derive(Discard)]
export struct Out { export i: In }

fn main() { let _o = Out { i: In {} }; }
// error: field `i` is exported, but its type names `In`, which isn't exported
