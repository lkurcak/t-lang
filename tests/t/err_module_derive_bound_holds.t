// What a derive says is what holds, even if the fields need less: `Later
// <Fd>` isn't `Discard`, so it can't be thrown away.
mod cells {
    #[derive(Discard where T: Discard)]
    pub struct Later<T> { of: fn() -> T }

    pub fn later<T>(of: fn() -> T) -> Later<T> { Later { of } }

    pub struct Fd { n: i64 }

    pub fn fd() -> Fd { Fd { n: 1 } }
}

fn main() {
    let _l = cells::later(cells::fd);
}
// error: `_l` is never used up
