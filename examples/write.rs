#[macro_use]
extern crate f77_io;

fn main() {
    f77_write!(*, "(I8.3, F8.3)", 42u64, 123.456)
        .expect("failed to write u64 and a float");
    let ary = vec![1u32,2,3,4];
    f77_write!(*, "(I8.3)", ary)
        .expect("failed to write an array");
    f77_write!(*, *, 42u32, 123.456f32)
        .expect("failed to write star format");
    f77_write!(*, *, ary)
        .expect("failed to write star array");
}
