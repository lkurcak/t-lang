// A range whose start is after its end is empty; written with constants,
// it's most likely a countdown that won't run.
fn main() {
    for i in 5..0 {
        print(i);
    }
    print(1); // expect: 1
}
// warning: this range is empty: `5` is after `0`, so it never yields anything
