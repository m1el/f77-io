#[macro_use]
extern crate f77_io;

fn main() {
    use std::io::BufReader;
    {
        let input = "1\n";
        let buffer = BufReader::new(input.as_bytes());
        let mut i = 0i32;
        println!("input: {:?}", input);
        f77_read!(buffer, *, i)
            .expect("could not read int in default editing");
        println!("values: {}\n", i);
    }

    {
        let input = "1\n\n\n2";
        let buffer = BufReader::new(input.as_bytes());
        let mut a = 0i32;
        let mut b = 0i32;
        f77_read!(buffer, *, a, b)
            .expect("could not read multiple ints with empty lines between");
        println!("input: {:?}", input);
        println!("values: {}, {}\n", a, b);
    }

    {
        let input = "1,2\n3\ntrailing";
        let buffer = BufReader::new(input.as_bytes());
        let mut a = 0i32;
        let mut b = 0i32;
        let mut c = 0i32;
        f77_read!(buffer, *, a, b, c)
            .expect("could not read multiple ints using default editing with trailing characters");
        println!("input: {:?}", input);
        println!("values: {}, {}, {}\n", a, b, c);
    }

    {
        let input = "first line to read\nsecond line to read\ntrailing input";
        let buffer = BufReader::new(input.as_bytes());
        let mut s1 = String::new();
        let mut s2 = String::new();
        f77_read!(buffer, *, s1, s2)
            .expect("coult not read two strings");
        println!("input: {:?}", input);
        println!("values: {:?}, {:?}\n", s1, s2);
    }
}
