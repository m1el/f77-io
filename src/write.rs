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
            },
            iter: fmt.into_iter().peekable(),
        }
    }

    fn is_const(n: &FormatNode) -> Result<bool, ()> {
        use format::FormatNode::*;
        let rv = match n {
            &NewLine => true,
            &SkipChar => true,
            &SuppressNewLine => true,
            &Terminate => true,
            &BlankControl(_) => true,
            &AbsColumn(_) => true,
            &RelColumn(_) => true,
            &Radix(_) => true,
            &Scale(_) => true,
            &Literal(_) => true,

            &Str(_) => false,
            &Bool(_) => false,
            &Int(_, _) => false,
            &Oct(_, _) => false,
            &Hex(_, _) => false,
            &Real(_, _, _, _) => false,
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
            let cont = match self.iter.peek() {
                Some(x) => try!(Self::is_const(*x)),
                None => false,
            };
            if !cont {
                break;
            }

            let next = self.iter.next().unwrap();
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
        let n = match self.iter.next() {
            Some(n) => n,
            None => return Err(()),
        };
        dst.write_all(val.f77_format(n, &self.opts).as_bytes())
            .map_err(|_|())
    }
}
