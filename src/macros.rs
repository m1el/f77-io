#[macro_export]
macro_rules! f77_write {
    (*, $src: expr, $($val: expr),*) => {{
        let mut stdout = std::io::stdout();
        f77_write!(stdout, $src, $($val),*)
    }};
    ($out: expr, $src: expr, $($val: expr),*) => {{
        let fmt = $crate::format::parse_format($src).expect("Could not parse format string");
        let out = &mut $out;
        let mut writer = $crate::write::FortranIterWriter::new(&fmt);
        (move || -> Result<(), $crate::write::WriteErr> {
            $(
                try!(writer.write_constants(out, true));
                try!(writer.write_value(out, &$val));
            )*
            try!(writer.write_constants(out, false));
            Ok(())
        })()
    }}
}
