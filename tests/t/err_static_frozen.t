// A static is a place that can change: one of a frozen type couldn't, and
// is what a const is for.
static LIMIT: u32 = 10;
// error: a static of a frozen type, `u32`, never changes: make it a const

fn main() {}
