use std::io::{Write};
use std::iter::{Peekable};
use format::*;
use types::*;
use iter::*;

pub struct WriterOpts {
    terminated: bool,
    suppress_newline: bool,
    scale: isize,
    radix: usize,
}

pub struct FortranIterWriter<'a> {
    iter: Peekable<FormatEvalIter<'a>>,
    node: &'a FormatNode,
    opts: WriterOpts,
    consumed_data: bool,
}


#[derive(Debug)]
pub enum WriteErr {
    IoErr(::std::io::Error),
    DataWithoutFormat,
    UnexpectedQInWrite,
    InvalidState,
    InvalidEditing(FormatNode, FortranTag),
}

impl From<::std::io::Error> for WriteErr {
    fn from(x: ::std::io::Error) -> WriteErr {
        WriteErr::IoErr(x)
    }
}

pub trait FortranWrite {
    fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr>;
    fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr>;
}

macro_rules! impl_bool_write {
    ($ty: ty) => {
        impl FortranWrite for $ty {
            fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
                let n = match writer.iter.next() {
                    Some(n) if requires_data(n)? => n,
                    _ => return Err(WriteErr::InvalidState),
                };
                let val = *self;

                let ow =
                    if let &FormatNode::Bool(ow) = n { ow }
                    else { return Err(WriteErr::InvalidState) };
                let c = if val { 'T' } else { 'F' };
                write!(dst, "{:>w$}", c, w=ow.unwrap_or(2))?;
                Ok(())
            }

            fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
                let c = if *self { 'T' } else { 'F' };
                write!(dst, "{:>7}", c)?;
                Ok(())
            }
        }
    }
}

impl_bool_write!(bool);

macro_rules! impl_int_write {
    ($ty: ty, $w: expr) => {
        impl FortranWrite for $ty {
            fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
                let n = match writer.iter.next() {
                    Some(n) if try!(requires_data(n)) => n,
                    _ => return Err(WriteErr::InvalidState),
                };
                writer.consumed_data = true;
                let val = *self;

                let (t, w, om) =
                    if let &FormatNode::Int(t, w, om) = n { (t, w, om) }
                    else { return Err(WriteErr::InvalidState) };

                let mut s = if let Some(m) = om {
                    match t {
                        IntFormat::I => format!("{:>w$}", format!("{:0m$}", val, m=m), w=w),
                        IntFormat::O => format!("{:>w$}", format!("{:0m$o}", val, m=m), w=w),
                        IntFormat::Z => format!("{:>w$}", format!("{:0m$x}", val, m=m), w=w),
                    }
                } else {
                    match t {
                        IntFormat::I => format!("{:>w$}", val, w=w),
                        IntFormat::O => format!("{:>w$o}", val, w=w),
                        IntFormat::Z => format!("{:>w$x}", val, w=w),
                    }
                };

                if s.len() > w {
                    s = format!("{:*>w$}", w=w)
                }
                write!(dst, "{}", s)?;
                Ok(())
            }

            fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
                const W: usize = $w;
                write!(dst, "{:>w$}", *self, w=W)?;
                Ok(())
            }
        }
    }
}

impl_int_write! { i64, 22 }
impl_int_write! { i32, 12 }
impl_int_write! { i16, 7 }
impl_int_write! { i8, 5 }
impl_int_write! { u64, 22 }
impl_int_write! { u32, 12 }
impl_int_write! { u16, 7 }
impl_int_write! { u8, 5 }

macro_rules! impl_float_write {
    ($ty: ty, $w: expr, $d: expr, $e: expr) => {
        impl FortranWrite for $ty {
            fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
                let n = match writer.iter.next() {
                    Some(n) if try!(requires_data(n)) => n,
                    _ => return Err(WriteErr::InvalidState),
                };
                writer.consumed_data = true;
                let val = *self;

                let (_t, w, od) =
                    if let &FormatNode::Real(t, w, od, _oe) = n { (t, w, od) }
                    else { return Err(WriteErr::InvalidState) };

                let s = if let Some(d) = od {
                    format!("{:>w$.d$}", val, w=w, d=d)
                } else {
                    format!("{:>w$}", val, w=w)
                };
                if s.len() > w {
                    write!(dst, "{:*>width$}", "", width=w)?;
                } else {
                    write!(dst, "{:>width$}", s, width=w)?;
                }
                Ok(())
            }

            fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
                const W: usize = $w;
                const D: usize = $d;
                write!(dst, "{: >w$.d$}", *self, w=W, d=D)?;
                Ok(())
            }
        }
    }
}

impl_float_write! { f64, 25, 16, 2 }
impl_float_write! { f32, 15, 6, 2 }

impl FortranWrite for String {
    fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
        let n = match writer.iter.next() {
            Some(n) if try!(requires_data(n)) => n,
            _ => return Err(WriteErr::InvalidState),
        };
        writer.consumed_data = true;

        let ow =
            if let &FormatNode::Str(ow) = n { ow }
            else { return Err(WriteErr::InvalidState) };
        if let Some(w) = ow {
            let len = self.len();
            if len <= w {
                write!(dst, "{:>w$}", self, w=w)?;
            } else {
                let s: String = self.chars().take(w).collect();
                write!(dst, "{}", s)?;
            }
        } else {
            write!(dst, "{}", self)?;
        }
        Ok(())
    }

    fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
        dst.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl<'a, T: FortranWrite> FortranWrite for &'a [T] {
    fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
        for v in self.iter() {
            writer.write_constants(dst, true)?;
            (&v).fortran_write(dst, writer)?;
        }
        Ok(())
    }

    fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
        for v in self.iter() {
            (&v).fortran_write_default(dst)?;
        }
        Ok(())
    }
}

impl<T: FortranWrite> FortranWrite for Vec<T> {
    fn fortran_write<W: Write>(&self, dst: &mut W, writer: &mut FortranIterWriter) -> Result<(), WriteErr> {
        for v in self.iter() {
            writer.write_constants(dst, true)?;
            (&v).fortran_write(dst, writer)?;
        }
        Ok(())
    }

    fn fortran_write_default<W: Write>(&self, dst: &mut W) -> Result<(), WriteErr> {
        for v in self.iter() {
            (&v).fortran_write_default(dst)?;
        }
        Ok(())
    }
}

fn requires_data(n: &FormatNode) -> Result<bool, WriteErr> {
    use format::FormatNode::*;
    let rv = match n {
        &NewLine => false,
        &SkipChar => false,
        &SuppressNewLine => false,
        &Terminate => false,
        &BlankControl(_) => false,
        &AbsColumn(_) => false,
        &RelColumn(_) => false,
        &Radix(_) => false,
        &Scale(_) => false,
        &Literal(_) => false,

        &Str(_) => true,
        &Bool(_) => true,
        &Int(_, _, _) => true,
        &Real(_, _, _, _) => true,
        &Group(_) | &Repeat(_, _) => unreachable!(),
        &RemainingChars => return Err(WriteErr::UnexpectedQInWrite),
    };
    Ok(rv)
}

impl<'a> FortranIterWriter<'a> {
    pub fn new<'f>(fmt: &'f FormatNode) -> FortranIterWriter<'f> {
        FortranIterWriter {
            opts: WriterOpts {
                terminated: false,
                suppress_newline: false,
                scale: 0,
                radix: 10,
            },
            consumed_data: false,
            node: fmt,
            iter: fmt.into_iter().peekable(),
        }
    }


    pub fn write_constants<W>(&mut self, dst: &mut W, has_data: bool) -> Result<(), WriteErr>
        where W: Write
    {
        use format::FormatNode::*;
        loop {
            let has_next = self.iter.peek().is_some();

            if !has_next {
                // a the end of the iterator
                if !has_data {
                    // with no data, print the newline, done
                    if !self.opts.suppress_newline {
                        dst.write_all(b"\n")?;
                    }
                    return Ok(())
                } else {
                    // if there's data present, but the format string
                    // consumes no data, this is an error
                    if !self.consumed_data {
                        return Err(WriteErr::DataWithoutFormat);
                    } else {
                        // otherwise, we've reached the end of the pattern, reset the iterator
                        self.iter = self.node.into_iter().peekable();
                    }
                }
            }

            if let Some(next) = self.iter.peek() {
                if try!(requires_data(*next)) {
                    break;
                }
            }

            let next = self.iter.next().unwrap();

            match next {
                &Radix(r) => { self.opts.radix = r; },
                &Scale(p) => { self.opts.scale = p; },
                &Literal(ref s) => {
                    try!(dst.write_all(s.as_bytes()));
                } ,
                // TODO: seek until next newline?
                &NewLine => {
                    try!(dst.write_all(b"\n"));
                },
                // TODO: seek instead of writing space?
                &SkipChar => {
                    try!(dst.write_all(b" "));
                },
                &SuppressNewLine => {
                    self.opts.suppress_newline = true;
                },
                &Terminate => {
                    if !has_data {
                        self.opts.terminated = true;
                        return Ok(());
                    }
                },
                // TODO: seek?
                &BlankControl(_) => {},
                &AbsColumn(_) => {},
                &RelColumn(_) => {},
                x@_ => {
                    unreachable!(format!("{:?}", x))
                }
            }
        }
        Ok(())
    }

    pub fn write_value<W: Write, T: FortranWrite>
        (&mut self, dst: &mut W, val: &T) -> Result<(), WriteErr>
    {
        val.fortran_write(dst, self)
    }
}
