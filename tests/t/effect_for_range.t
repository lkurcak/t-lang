// Counting through a range can't panic: `Range::next` checks before it
// adds one. The sum can.
fn sum(n: i64) -> i64 with panic {
    let mut s = 0;
    for i in 0..n {
        s = s + i;
    }
    s
}

fn count(n: i64) -> bool {
    for _i in 0..n {
        return true;
    }
    false
}

fn main() with panic {
    print(sum(4)); // expect: 6
    print_bool(count(3)); // expect: true
}
