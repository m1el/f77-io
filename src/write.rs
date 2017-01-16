use std::io::{Write};
use std::iter::{Peekable};
use format::*;
use iter::*;

pub struct WriterOpts {
    terminated: bool,
    suppress_newline: bool,
    scale: isize,
    radix: usize,
    wants_data: bool,
    consumed_data: bool,
    has_something: bool,
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
}

pub trait FortranFormat {
    fn f77_format(&self, fmt: &FormatNode, opts: &WriterOpts) -> String;
}

impl<'a> FortranIterWriter<'a> {
    pub fn new<'f>(fmt: &'f FormatNode) -> FortranIterWriter<'f> {
        FortranIterWriter {
            opts: WriterOpts {
                terminated: false,
                suppress_newline: false,
                scale: 0,
                radix: 10,
                consumed_data: false,
                wants_data: true,
                has_something: false,
            },
            node: fmt,
            iter: fmt.into_iter().peekable(),
        }
    }

    fn requires_data(n: &FormatNode) -> Result<bool, ()> {
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
            &RemainingChars => return Err(()),
        };
        Ok(rv)
    }

    pub fn write_constants<W>(&mut self, dst: &mut W, has_data: bool) -> Result<(), ()>
        where W: Write
    {
        use format::FormatNode::*;
        loop {
            if self.iter.peek().is_none() {
                if !self.opts.consumed_data && has_data {
                    return Err(());
                }
                if !self.opts.has_something {
                    return Ok(());
                }
                // we've reached the end of the pattern, reset the iterator
                self.iter = self.node.into_iter().peekable();
            }

            let next = match self.iter.next() {
                None => break,
                Some(x) => {
                    self.opts.has_something = true;
                    if try!(Self::requires_data(x)) {
                        self.opts.wants_data = true;
                        break;
                    }
                    x
                }
            };

            match next {
                &Radix(r) => { self.opts.radix = r; },
                &Scale(p) => { self.opts.scale = p; },
                &Literal(ref s) => {
                    try!(dst.write_all(s.as_bytes()).map_err(|_|()));
                } ,
                &NewLine => {
                    try!(dst.write_all(b"\n").map_err(|_|()));
                },
                &SkipChar => {
                    try!(dst.write_all(b" ").map_err(|_|()));
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

    pub fn write_value<W, T>(&mut self, dst: &mut W, val: &T) -> Result<(), ()>
        where W: Write, T: FortranFormat
    {
        if !self.opts.wants_data {
            return Err(());
        }
        let n = match self.iter.next() {
            Some(n) => n,
            None => return Err(()),
        };
        self.opts.has_something = true;
        self.opts.consumed_data = true;
        self.opts.wants_data = false;
        dst.write_all(val.f77_format(n, &self.opts).as_bytes()).map_err(|_|())
    }

    pub fn write_ary<W, T>(&mut self, dst: &mut W, ary: &[T]) -> Result<(), ()>
        where W: Write, T: FortranFormat
    {
        for val in ary.iter() {
            if !self.opts.wants_data {
                try!(self.write_constants(dst, true).map_err(|_|()));
            }
            try!(self.write_value(dst, val).map_err(|_|()));
        }
        Ok(())
    }
}
