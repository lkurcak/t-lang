fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"Hello, T!") // expect: Hello, T!
}
