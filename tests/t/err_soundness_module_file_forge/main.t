// An abstract type from a file can't be forged outside it, any more than
// one from an inline module.
mod fd;
fn main() {
    let f = fd::Fd { raw: 3 }; // error: field `raw` of `Fd` is private to `fd`
    print(fd::close(f));
}
