mod m {
    #[derive(Discard)]
    enum Hidden {
        A,
    }
}

use m::Hidden::A;
// error: `Hidden` is private to `m`

fn main() {}
