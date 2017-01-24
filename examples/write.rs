#[macro_use]
extern crate f77_io;

fn main() {
    f77_write!(*, "(I8.3, F8.3)", 42, 123.456).unwrap();
}
