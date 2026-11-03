// So is `Send`.
#[derive(Discard)]
struct Token {}

impl Send for Token {}
// error: `Send` can only be derived

fn main() {}
