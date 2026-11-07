// Writing to a sink does what the sink does: a function generic over one
// says so, as `List<u8>`'s allocates: an exported one, whose effects
// aren't inferred.
use core::io::Write;

export fn hello<W: Write>(w: &mut W) -> Result<(), std::io::Error> {
    w.write_all("hello".as_bytes()) // error: but `hello` doesn't say `with W::e`
}

fn main() {}
