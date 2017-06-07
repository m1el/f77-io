#[macro_export]
macro_rules! f77_write_star {
    ($out: expr, $($val: expr),*) => {{
        use ::std::io::Write;
        use $crate::write::{FortranWrite};
        Ok(())
        $(
            .and_then(|_| FortranWrite::fortran_write_default(&$val, $out))
        )*
            .and_then(|_| $out.write_all(b"\n").map_err(|e|e.into()))
    }}
}

#[macro_export]
macro_rules! f77_write {
    (*, *, $($val: expr),*) => {{
        let mut stdout = ::std::io::stdout();
        f77_write!(&mut stdout, *, $($val),*)
    }};

    (*, $out: expr, $($val: expr),*) => {{
        let mut stdout = ::std::io::stdout();
        f77_write!(&mut stdout, $out, $($val),*)
    }};

    ($out: expr, *, $($val: expr),*) => {{
        f77_write_star!($out, $($val),*)
    }};

    ($out: expr, $src: expr, $($val: expr),*) => {{
        let fmt = $crate::format::parse_format($src).expect("Could not parse format string");
        let out = &mut $out;
        let mut writer = $crate::write::FortranIterWriter::new(&fmt);
        Ok(())
        $(
            .and_then(|_| writer.write_constants(out, true))
            .and_then(|_| writer.write_value(out, &$val))
        )*
            .and_then(|_| writer.write_constants(out, false))
    }}
}

#[macro_export]
macro_rules! f77_read_star {
    ($inp: expr, $($val: expr),*) => {{
        let mut reader = $crate::read::FortranDefaultReader::new(&mut $inp);
        Ok(())
        $(
            .and_then(|_| reader.read_value(&mut $val))
        )*
    }}
}

#[macro_export]
macro_rules! f77_read {
    (*, *, $($val: expr),*) => {{
        use ::std::io::BufReader;
        let mut stdin = BufReader::new(::std::io::stdin());
        f77_read!(stdin, *, $($val),*)
    }};

    (*, $inp: expr, $($val: expr),*) => {{
        use ::std::io::BufReader;
        let mut stdin = BufReader::new(::std::io::stdin());
        f77_read!(stdin, $src, $($val),*)
    }};

    ($inp: expr, *, $($val: expr),*) => {{
        f77_read_star!($inp, $($val),*)
    }};

    ($inp: expr, $src: expr, $($val: expr),*) => {{
        let fmt = $crate::format::parse_format($src).expect("Could not parse format string");
        let inp = &mut $inp;
        let mut reader = $crate::write::FortranIterReader::new(&fmt, &mut $inp);
        Ok(())
        $(
            .and_then(|_| reader.read_constants(inp, true))
            .and_then(|_| reader.read_value(inp, &mut $val))
        )*
            .and_then(|_| reader.read_constants(inp, false))
    }};
}
