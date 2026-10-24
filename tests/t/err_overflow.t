fn main() with panic {
    let _x: i64 = 9223372036854775807 + 1;
}
// error: runtime error: integer overflow
