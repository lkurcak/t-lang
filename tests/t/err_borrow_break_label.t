// `break 'outer` leaves both loops, and `y`, in the outer one's body, goes
// away with them.
fn main() {
    let r;
    'outer: loop {
        let y = 5;
        loop {
            r = &y;
            break 'outer;
        }
    }
    print(*r);
}
// error: `y` doesn't live long enough
