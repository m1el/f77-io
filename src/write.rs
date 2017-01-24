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

#[cfg(never)]
pub struct FortranVmWriter<'a> {
    vm: Peekable<FormatVmIter>,
    opts: WriterOpts,
}

pub struct FortranIterWriter<'a> {
    iter: Peekable<FormatEvalIter<'a>>,
    node: &'a FormatNode,
    opts: WriterOpts,
    consumed_data: bool,
}

pub trait FortranFormat {
    fn f77_format(&self, fmt: &FormatNode, opts: &WriterOpts) -> Result<String, WriteErr>;
}

impl FortranFormat for bool {
    fn f77_format(&self, fmt: &FormatNode, _: &WriterOpts) -> Result<String, WriteErr> {
        match fmt {
            &FormatNode::Bool(ow) => {
                let c = if *self { 'T' } else { 'F' };
                let w = ow.unwrap_or(2);
                Ok(format!("{:>w$}", c, w=w))
            },
            _ => Err(WriteErr::InvalidEditing(fmt.clone(), Self::fortran_tag())),
        }
    }
}

macro_rules! impl_bool {
    ($t: ident) => {
        impl FortranFormat for $t {
            fn f77_format(&self, fmt: &FormatNode, opts: &WriterOpts) -> Result<String, WriteErr> {
                bool::from(*self).f77_format(fmt, opts)
            }
        }
    }
}
impl_bool!(Fbool2);
impl_bool!(Fbool4);
impl_bool!(Fbool8);

macro_rules! impl_int {
    ($t: ident, $w: expr) => {
        impl FortranFormat for $t {
            fn f77_format(&self, fmt: &FormatNode, _: &WriterOpts) -> Result<String, WriteErr> {
                match fmt {
                    &FormatNode::Int(ow, om) => {
                        let w = ow.unwrap_or($w);
                        let mut s = if let Some(m) = om {
                            format!("{:>w$}", format!("{:0m$}", *self, m=m), w=w)
                        } else {
                            format!("{:>w$}", *self, w=w)
                        };
                        if s.len() > w {
                            s = format!("{:*>w$}", w=w)
                        }
                        Ok(s)
                    },
                    _ => Err(WriteErr::InvalidEditing(fmt.clone(), Self::fortran_tag())),
                }
            }
        }
    }
}
impl_int!(i8, 7);
impl_int!(i16, 7);
impl_int!(u16, 7);
impl_int!(i32, 12);
impl_int!(u32, 12);
impl_int!(i64, 23);
impl_int!(u64, 23);

macro_rules! impl_float {
    ($t: ident, $w: expr) => {
        impl FortranFormat for $t {
            fn f77_format(&self, fmt: &FormatNode, _: &WriterOpts) -> Result<String, WriteErr> {
                match fmt {
                    &FormatNode::Real(ref _f, ow, od, _oe) => {
                        let w = ow.unwrap_or($w);
                        let mut s = if let Some(d) = od {
                            format!("{:>w$.d$}", *self, w=w, d=d)
                        } else {
                            format!("{:>w$}", *self, w=w)
                        };
                        if s.len() > w {
                            s = format!("{:*>width$}", "", width=w)
                        } else {
                            s = format!("{:>width$}", s, width=w)
                        }
                        Ok(s)
                    },
                    _ => Err(WriteErr::InvalidEditing(fmt.clone(), Self::fortran_tag())),
                }
            }
        }
    }
}
impl_float!(f32, 12);
impl_float!(f64, 25);

impl FortranFormat for String {
    fn f77_format(&self, fmt: &FormatNode, _: &WriterOpts) -> Result<String, WriteErr> {
        match fmt {
            &FormatNode::Str(ow) => {
                let rv = if let Some(w) = ow {
                    let len = self.len();
                    if len < w {
                        format!("{:*>w$}", "", w=w)
                    } else if len > w {
                        self.chars().take(w).collect()
                    } else {
                        self.clone()
                    }
                } else {
                    self.clone()
                };
                Ok(rv)
            },
            _ => Err(WriteErr::InvalidEditing(fmt.clone(), Self::fortran_tag())),
        }
    }
}

#[derive(Debug)]
pub enum WriteErr {
    IoErr(::std::io::Error),
    DataWithoutFormat,
    UnexpectedQInRead,
    InvalidState,
    InvalidEditing(FormatNode, FortranTag),
}

macro_rules! ioerr {
    ($x: expr) => { $x.map_err(|x| WriteErr::IoErr(x)) }
}

macro_rules! tryio {
    ($x: expr) => { try!(ioerr!($x)) }
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
            &Int(_, _) => true,
            &Oct(_, _) => true,
            &Hex(_, _) => true,
            &Real(_, _, _, _) => true,
            &Group(_) | &Repeat(_, _) => unreachable!(),
            &RemainingChars => return Err(WriteErr::UnexpectedQInRead),
        };
        Ok(rv)
    }

    pub fn write_constants<W>(&mut self, dst: &mut W, has_data: bool) -> Result<(), WriteErr>
        where W: Write
    {
        use format::FormatNode::*;
        loop {
            let has_next =
                if let Some(next) = self.iter.peek() {
                    if try!(Self::requires_data(*next)) {
                        break;
                    }
                    true
                } else {
                    false
                };

            if !has_next {
                // a the end of the iterator
                if !has_data {
                    // with no data, print the newline, done
                    if !self.opts.suppress_newline {
                        return ioerr!(dst.write_all(b"\n"));
                    } else {
                        return Ok(());
                    }
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

            let next = self.iter.next().unwrap();

            match next {
                &Radix(r) => { self.opts.radix = r; },
                &Scale(p) => { self.opts.scale = p; },
                &Literal(ref s) => {
                    tryio!(dst.write_all(s.as_bytes()));
                } ,
                // TODO: seek until next newline?
                &NewLine => {
                    tryio!(dst.write_all(b"\n"));
                },
                // TODO: seek instead of writing space?
                &SkipChar => {
                    tryio!(dst.write_all(b" "));
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

    pub fn write_value<W, T>(&mut self, dst: &mut W, val: &T) -> Result<(), WriteErr>
        where W: Write, T: FortranFormat
    {
        let n = match self.iter.next() {
            Some(n) if try!(Self::requires_data(n)) => n,
            _ => return Err(WriteErr::InvalidState),
        };
        self.consumed_data = true;
        let s = try!(val.f77_format(n, &self.opts));
        ioerr!(dst.write_all(s.as_bytes()))
    }

    pub fn write_ary<W, T>(&mut self, dst: &mut W, ary: &[T]) -> Result<(), WriteErr>
        where W: Write, T: FortranFormat
    {
        for val in ary.iter() {
            try!(self.write_constants(dst, true));
            try!(self.write_value(dst, val));
        }
        Ok(())
    }
}
