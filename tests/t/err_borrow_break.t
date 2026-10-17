// A `break` leaves the loop's body, and `y` goes away with it, so the
// reference it gives the loop would dangle.
fn main() {
    let r = loop {
        let y = 5;
        break &y;
    };
    print(*r);
}
// error: `y` doesn't live long enough: it goes out of scope here while still borrowed
