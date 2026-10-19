mod m {
    #[derive(Discard)]
    struct Hidden { n: int }

    pub fn n() -> int { 1 }
}

fn f(h: m::Hidden) -> int { m::n() }

fn main() {}
// error: `Hidden` is private to `m`
