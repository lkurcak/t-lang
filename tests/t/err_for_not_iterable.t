fn main() {
    for x in 5i64 { // error: `i64` can't be iterated: it isn't an iterator, nor `IntoIterator`
        let _y = x;
    }
}
