// A struct can't be generic over effects: what it holds has types, and a
// function type in it says its effects itself.
struct W<e> { n: int }

fn main() {}
// error: a struct can't have effect parameters
