fn main() {
    'outer: loop {
        loop {
            break 'outr;
        }
    }
}
// error: no loop around this `break` is labelled `'outr`
