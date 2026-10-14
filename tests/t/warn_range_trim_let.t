// Irrefutable, but the second alternative only ever matches `0..`.
fn main() {
    let ..0 | -5.. = 3;
}
// warning: this range can be narrowed to `0..`
