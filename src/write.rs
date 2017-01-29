use std::io::{Write};
use std::iter::{Peekable};
use std::any::{Any};
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


#[derive(Debug)]
pub enum WriteErr {
    IoErr(::std::io::Error),
    DataWithoutFormat,
    UnexpectedQInWrite,
    InvalidState,
    InvalidEditing(FormatNode, FortranTag),
}

macro_rules! ioerr {
    ($x: expr) => { $x.map_err(|x| WriteErr::IoErr(x)) }
}

macro_rules! tryio {
    ($x: expr) => { try!(ioerr!($x)) }
}

fn write_bool<W: Write>(
    dst: &mut W, val: bool,
    ow: Option<usize>)
    -> Result<(), WriteErr>
{
    let c = if val { 'T' } else { 'F' };
    ioerr!(write!(dst, "{:>w$}", c, w=ow.unwrap_or(2)))
}

fn write_u64<W: Write>(
    dst: &mut W, t: IntFormat,
    val: u64, w: usize, om: Option<usize>)
    -> Result<(), WriteErr>
{
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
    ioerr!(write!(dst, "{}", s))
}

fn write_i64<W: Write>(
    dst: &mut W, t: IntFormat,
    val: i64, w: usize, om: Option<usize>)
    -> Result<(), WriteErr>
{
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
    ioerr!(write!(dst, "{}", s))
}

fn write_f32<W: Write>(
    dst: &mut W, val: f32, _t: RealFormat,
    ow: Option<usize>, od: Option<usize>, _oe: Option<usize>)
    -> Result<(), WriteErr>
{
    let w = ow.unwrap_or(12);
    let s = if let Some(d) = od {
        format!("{:>w$.d$}", val, w=w, d=d)
    } else {
        format!("{:>w$}", val, w=w)
    };
    if s.len() > w {
        ioerr!(write!(dst, "{:*>width$}", "", width=w))
    } else {
        ioerr!(write!(dst, "{:>width$}", s, width=w))
    }
}

fn write_f64<W: Write>(
    dst: &mut W, val: f64, _t: RealFormat,
    ow: Option<usize>, od: Option<usize>, _oe: Option<usize>)
    -> Result<(), WriteErr>
{
    let w = ow.unwrap_or(23);
    let s = if let Some(d) = od {
        format!("{:>w$.d$}", val, w=w, d=d)
    } else {
        format!("{:>w$}", val, w=w)
    };
    if s.len() > w {
        ioerr!(write!(dst, "{:*>width$}", "", width=w))
    } else {
        ioerr!(write!(dst, "{:>width$}", s, width=w))
    }
}

fn write_str<W: Write>(
    dst: &mut W, val: &str,
    ow: Option<usize>)
    -> Result<(), WriteErr>
{
    if let Some(w) = ow {
        let len = val.len();
        if len <= w {
            ioerr!(write!(dst, "{:>w$}", val, w=w))
        } else {
            let s: String = val.chars().take(w).collect();
            ioerr!(write!(dst, "{}", s))
        }
    } else {
        ioerr!(write!(dst, "{}", val))
    }
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
            &Int(_, _, _) => true,
            &Real(_, _, _, _) => true,
            &Group(_) | &Repeat(_, _) => unreachable!(),
            &RemainingChars => return Err(WriteErr::UnexpectedQInWrite),
        };
        Ok(rv)
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

            if let Some(next) = self.iter.peek() {
                if try!(Self::requires_data(*next)) {
                    break;
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

    pub fn write_ary<W: Write, T: Any + FortranAltType>
        (&mut self, dst: &mut W, vals: &Vec<T>) -> Result<(), WriteErr>
    {
        for val in vals.iter() {
            try!(self.write_constants(dst, true));
            try!(self.write_value(dst, val));
        }
        Ok(())
    }

    pub fn write_value<W: Write, T: Any + FortranAltType>
        (&mut self, dst: &mut W, val: &T) -> Result<(), WriteErr>
    {
        let n = match self.iter.next() {
            Some(n) if try!(Self::requires_data(n)) => n,
            _ => return Err(WriteErr::InvalidState),
        };
        self.consumed_data = true;
        let typ = T::fortran_type();
        let tag = typ.tag;
        let val = val as &Any;

        use self::WriteErr::*;
        use format::FormatNode::*;
        if let Some(_dim) = typ.dim {
            match tag {
                FortranTag::Bool => self.write_ary(dst, val.downcast_ref::<Vec<bool>>().unwrap()),
                FortranTag::Bool2 => self.write_ary(dst, val.downcast_ref::<Vec<Fbool2>>().unwrap()),
                FortranTag::Bool4 => self.write_ary(dst, val.downcast_ref::<Vec<Fbool4>>().unwrap()),
                FortranTag::Bool8 => self.write_ary(dst, val.downcast_ref::<Vec<Fbool8>>().unwrap()),
                FortranTag::Byte => self.write_ary(dst, val.downcast_ref::<Vec<i8>>().unwrap()),
                FortranTag::Int2 => self.write_ary(dst, val.downcast_ref::<Vec<i16>>().unwrap()),
                FortranTag::Int4 => self.write_ary(dst, val.downcast_ref::<Vec<i32>>().unwrap()),
                FortranTag::Int8 => self.write_ary(dst, val.downcast_ref::<Vec<i64>>().unwrap()),
                FortranTag::Uint2 => self.write_ary(dst, val.downcast_ref::<Vec<u16>>().unwrap()),
                FortranTag::Uint4 => self.write_ary(dst, val.downcast_ref::<Vec<u32>>().unwrap()),
                FortranTag::Uint8 => self.write_ary(dst, val.downcast_ref::<Vec<u64>>().unwrap()),
                FortranTag::Real4 => self.write_ary(dst, val.downcast_ref::<Vec<f32>>().unwrap()),
                FortranTag::Real8 => self.write_ary(dst, val.downcast_ref::<Vec<f64>>().unwrap()),
                FortranTag::Complex4 => self.write_ary(dst, val.downcast_ref::<Vec<Complex<f32>>>().unwrap()),
                FortranTag::Complex8 => self.write_ary(dst, val.downcast_ref::<Vec<Complex<f64>>>().unwrap()),
                _ => Err(InvalidEditing(n.clone(), tag)),
            }
        } else {
            match n {
                &Bool(ow) => {
                    let b = match tag {
                        FortranTag::Bool => *val.downcast_ref::<bool>().unwrap(),
                        FortranTag::Bool2 => bool::from(*val.downcast_ref::<Fbool2>().unwrap()),
                        FortranTag::Bool4 => bool::from(*val.downcast_ref::<Fbool4>().unwrap()),
                        FortranTag::Bool8 => bool::from(*val.downcast_ref::<Fbool8>().unwrap()),
                        _ => return Err(InvalidEditing(n.clone(), tag)),
                    };
                    write_bool(dst, b, ow)
                },
                &Int(f, ow, om) => {
                    let oi = match tag {
                        FortranTag::Byte => Some((*val.downcast_ref::<i8>().unwrap() as i64, 7)),
                        FortranTag::Int2 => Some((*val.downcast_ref::<i16>().unwrap() as i64, 7)),
                        FortranTag::Int4 => Some((*val.downcast_ref::<i32>().unwrap() as i64, 12)),
                        FortranTag::Int8 => Some((*val.downcast_ref::<i64>().unwrap() as i64, 23)),
                        _ => None,
                    };
                    if let Some((i, dw)) = oi {
                        return write_i64(dst, f, i, ow.unwrap_or(dw), om);
                    }

                    let ou = match tag {
                        FortranTag::Uint2 => Some((*val.downcast_ref::<u16>().unwrap() as u64, 7)),
                        FortranTag::Uint4 => Some((*val.downcast_ref::<u32>().unwrap() as u64, 12)),
                        FortranTag::Uint8 => Some((*val.downcast_ref::<u64>().unwrap() as u64, 23)),
                        _ => None,
                    };
                    if let Some((u, dw)) = ou {
                        return write_u64(dst, f, u, ow.unwrap_or(dw), om);
                    }
                    Err(InvalidEditing(n.clone(), tag))
                },
                &Real(f, ow, od, oe) => {
                    match tag {
                        FortranTag::Real4 => {
                            let v = *val.downcast_ref::<f32>().unwrap();
                            write_f32(dst, v, f, ow, od, oe)
                        },
                        FortranTag::Real8 => {
                            let v = *val.downcast_ref::<f64>().unwrap();
                            write_f64(dst, v, f, ow, od, oe)
                        },
                        FortranTag::Complex4 => {
                            let v = *val.downcast_ref::<Complex<f32>>().unwrap();
                            tryio!(write!(dst, " ("));
                            try!(write_f32(dst, v.re, f, ow, od, oe));
                            tryio!(write!(dst, ","));
                            try!(write_f32(dst, v.im, f, ow, od, oe));
                            ioerr!(write!(dst, ")"))
                        }
                        FortranTag::Complex8 => {
                            let v = *val.downcast_ref::<Complex<f64>>().unwrap();
                            tryio!(write!(dst, " ("));
                            try!(write_f64(dst, v.re, f, ow, od, oe));
                            tryio!(write!(dst, ","));
                            try!(write_f64(dst, v.im, f, ow, od, oe));
                            ioerr!(write!(dst, ")"))
                        }
                        _ => Err(InvalidEditing(n.clone(), tag)),
                    }
                },
                &Str(ow) => {
                    match tag {
                        FortranTag::Strin => {
                            let v = val.downcast_ref::<String>().unwrap();
                            write_str(dst, v, ow)
                        },
                        _ =>  Err(InvalidEditing(n.clone(), tag)),
                    }
                }
                _ => Err(InvalidEditing(n.clone(), tag))
            }
        }
    }
}
