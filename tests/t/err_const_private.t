// A const is private to its module unless it's `pub`.
mod m {
    const SECRET: u8 = 3;
}

fn main() {
    let _ = m::SECRET;
// error: is private
}
