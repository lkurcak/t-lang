mod m {
    #[derive(Discard)]
    struct Hidden { n: i64 }

    pub fn n() -> i64 { 1 }
}

fn f(h: m::Hidden) -> i64 { m::n() }

fn main() {}
// error: `Hidden` is private to `m`
