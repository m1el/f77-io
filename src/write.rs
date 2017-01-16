use std::io::{Write};
use std::iter::{Peekable};
use format::*;
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
    wants_data: bool,
    consumed_data: bool,
    has_something: bool,
}

pub trait FortranFormat {
    fn f77_format(&self, fmt: &FormatNode, opts: &WriterOpts) -> String;
}

#[derive(Debug)]
pub enum WriteErr {
    IoErr(::std::io::Error),
    DataWithoutFormat,
    InvalidFormat,
    InvalidState,
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
            wants_data: true,
            has_something: false,
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
            &RemainingChars => return Err(WriteErr::InvalidFormat),
        };
        Ok(rv)
    }

    pub fn write_constants<W>(&mut self, dst: &mut W, has_data: bool) -> Result<(), WriteErr>
        where W: Write
    {
        use format::FormatNode::*;
        loop {
            if self.iter.peek().is_none() {
                if !self.consumed_data && has_data {
                    return Err(WriteErr::DataWithoutFormat);
                }
                if !self.has_something {
                    return ioerr!(dst.write_all(b"\n"));
                }
                // we've reached the end of the pattern, reset the iterator
                self.iter = self.node.into_iter().peekable();
            }

            let next = self.iter.next().unwrap();
            if try!(Self::requires_data(next)) {
                self.wants_data = true;
                break;
            }

            match next {
                &Radix(r) => { self.opts.radix = r; },
                &Scale(p) => { self.opts.scale = p; },
                &Literal(ref s) => {
                    tryio!(dst.write_all(s.as_bytes()));
                } ,
                &NewLine => {
                    tryio!(dst.write_all(b"\n"));
                },
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
                &BlankControl(_) => {},
                &AbsColumn(_) => {},
                &RelColumn(_) => {},
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    pub fn write_value<W, T>(&mut self, dst: &mut W, val: &T) -> Result<(), WriteErr>
        where W: Write, T: FortranFormat
    {
        if !self.wants_data {
            return Err(WriteErr::InvalidState);
        }
        let n = match self.iter.next() {
            Some(n) => n,
            None => return Err(WriteErr::InvalidState),
        };
        self.has_something = true;
        self.consumed_data = true;
        self.wants_data = false;
        ioerr!(dst.write_all(val.f77_format(n, &self.opts).as_bytes()))
    }

    pub fn write_ary<W, T>(&mut self, dst: &mut W, ary: &[T]) -> Result<(), WriteErr>
        where W: Write, T: FortranFormat
    {
        for val in ary.iter() {
            if !self.wants_data {
                try!(self.write_constants(dst, true));
            }
            try!(self.write_value(dst, val));
        }
        Ok(())
    }
}
