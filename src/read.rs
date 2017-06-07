use std::io::{BufRead};
use std::iter::{Peekable};
use format::*;
use types::*;
use iter::*;

pub struct ReaderOpts {
    terminated: bool,
    suppress_newline: bool,
    scale: isize,
    radix: usize,
}

pub struct FortranIterReader<'a, R: 'a+BufRead> {
    iter: Peekable<FormatEvalIter<'a>>,
    line: String,
    line_pos: usize,
    consumed_data: bool,
    read: &'a mut R,
    node: &'a FormatNode,
    opts: ReaderOpts,
}

pub struct FortranDefaultReader<'a, R: 'a+BufRead> {
    line: String,
    line_pos: usize,
    read: &'a mut R,
}

#[derive(Debug)]
pub enum ReadErr {
    IoErr(::std::io::Error),
    ParseIntError(::std::num::ParseIntError),
    ParseBoolError,
    UnexpectedLiteral,
    NoDataEditings,
    InvalidState,
    InvalidEditing(FormatNode, FortranTag),
}

impl From<::std::io::Error> for ReadErr {
    fn from(x: ::std::io::Error) -> ReadErr {
        ReadErr::IoErr(x)
    }
}

impl From<::std::num::ParseIntError> for ReadErr {
    fn from(x: ::std::num::ParseIntError) -> ReadErr {
        ReadErr::ParseIntError(x)
    }
}

pub trait FortranRead {
    fn fortran_read<R: BufRead>(&mut self, reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr>;
    fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr>;
}

macro_rules! impl_bool_read {
    ($ty: ty) => {
        impl FortranRead for $ty {
            fn fortran_read<R: BufRead>(&mut self, _reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr> {
                Ok(false)
            }

            fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr> {
                let next = reader.read_next_csv()?;
                let mut clear = next.chars().filter(|&c| !c.is_whitespace());
                match clear.next() {
                    Some('T') | Some('t') => {
                        *self = true;
                        Ok(true)
                    },
                    Some('F') | Some('f') => {
                        *self = false;
                        Ok(true)
                    },
                    None => Ok(false),
                    _ => Err(ReadErr::ParseBoolError),
                }
            }
        }
    }
}

impl_bool_read!(bool);

macro_rules! impl_int_read {
    ($ty: ty, $w: expr) => {
        impl FortranRead for $ty {
            fn fortran_read<R: BufRead>(&mut self, _reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr> {
                Ok(false)
            }

            fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr> {
                let next = reader.read_next_csv()?;
                let clear: String = next.chars().filter(|&c| !c.is_whitespace()).collect();
                if clear.len() != 0 {
                    *self = clear.parse()?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

impl_int_read! { i64, 22 }
impl_int_read! { i32, 12 }
impl_int_read! { i16, 7 }
impl_int_read! { i8, 5 }
impl_int_read! { u64, 22 }
impl_int_read! { u32, 12 }
impl_int_read! { u16, 7 }
impl_int_read! { u8, 5 }

macro_rules! impl_float_write {
    ($ty: ty, $w: expr, $d: expr, $e: expr) => {
        impl FortranRead for $ty {
            fn fortran_read(&self, reader: &mut FortranIterReader) -> Result<(), ReadErr> {
            }

            fn fortran_read_default(&self, reader: &mut FortranIterReader) -> Result<(), ReadErr> {
            }
        }
    }
}

//impl_float_write! { f64, 25, 16, 2 }
//impl_float_write! { f32, 15, 6, 2 }
//
impl FortranRead for String {
    fn fortran_read<R: BufRead>(&mut self, _reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr> {
        Ok(false)
    }

    fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr> {
        let next = reader.read_rest_string()?;
        *self = next;
        Ok(true)
    }
}

impl<'a, T: FortranRead> FortranRead for &'a mut [T] {
    fn fortran_read<R: BufRead>(&mut self, _reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr> {
        Ok(false)
    }

    fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr> {
        let mut read = false;
        for val in self.iter_mut() {
            if val.fortran_read_default(reader)? {
                read = true;
            }
        }
        Ok(read)
    }
}

impl<T: FortranRead> FortranRead for Vec<T> {
    fn fortran_read<R: BufRead>(&mut self, _reader: &mut FortranIterReader<R>) -> Result<bool, ReadErr> {
        Ok(false)
    }

    fn fortran_read_default<R: BufRead>(&mut self, reader: &mut FortranDefaultReader<R>) -> Result<bool, ReadErr> {
        let mut read = false;
        for val in self.iter_mut() {
            if val.fortran_read_default(reader)? {
                read = true;
            }
        }
        Ok(read)
    }
}

fn gives_data(n: &FormatNode) -> Result<bool, ReadErr> {
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
        &RemainingChars => true,
    };
    Ok(rv)
}

impl<'a, R: BufRead> FortranIterReader<'a, R> {
    pub fn new<'f>(fmt: &'f FormatNode, read: &'f mut R) -> FortranIterReader<'f, R> {
        FortranIterReader {
            opts: ReaderOpts {
                terminated: false,
                suppress_newline: false,
                scale: 0,
                radix: 10,
            },
            line: String::new(),
            line_pos: 0,
            read: read,
            consumed_data: false,
            node: fmt,
            iter: fmt.into_iter().peekable(),
        }
    }

    fn check_rest(&mut self) -> Result<bool, ReadErr> {
        if self.line_pos == self.line.len() {
            return Ok(self.read_line()?);
        }
        Ok(true)
    }

    fn read_line(&mut self) -> Result<bool, ReadErr> {
        self.line.clear();
        let read = self.read.read_line(&mut self.line)?;
        self.line_pos = 0;
        Ok(read != 0)
    }

    pub fn consume_constants(&mut self, want_data: bool) -> Result<(), ReadErr> {
        use format::FormatNode::*;
        loop {
            let has_next = self.iter.peek().is_some();

            if !has_next {
                // a the end of the iterator
                if !want_data {
                    // with no data, print the newline, done
                    if !self.opts.suppress_newline {
                        self.read_line()?;
                    }
                    return Ok(())
                } else {
                    // if we didn't read any data and we're
                    // at the end of the list, this is an error
                    if !self.consumed_data {
                        return Err(ReadErr::NoDataEditings);
                    } else {
                        // otherwise, we've reached the end of the pattern, reset the iterator
                        self.iter = self.node.into_iter().peekable();
                    }
                }
            }

            if let Some(next) = self.iter.peek() {
                if try!(gives_data(*next)) {
                    break;
                }
            }

            let next = self.iter.next().unwrap();

            match next {
                &Radix(r) => { self.opts.radix = r; },
                &Scale(p) => { self.opts.scale = p; },
                // TODO: seek until next newline?
                &NewLine => {
                    if !self.check_rest()? {
                        return Ok(());
                    }
                    self.line_pos = self.line.len();
                },
                // TODO: seek instead of writing space?
                &SkipChar => {
                    self.line_pos += 1;
                    if !self.check_rest()? {
                        return Ok(());
                    }
                },
                &SuppressNewLine => {},
                &Terminate => {
                    if !want_data {
                        self.opts.terminated = true;
                        return Ok(());
                    }
                },
                &Literal(_) => {
                    return Err(ReadErr::UnexpectedLiteral);
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

    pub fn read_value<T: FortranRead>(&mut self, val: &mut T) -> Result<bool, ReadErr> {
        val.fortran_read(self)
    }
}

impl<'a, R: BufRead> FortranDefaultReader<'a, R> {
    pub fn new<'f>(read: &'f mut R) -> FortranDefaultReader<'f, R> {
        FortranDefaultReader {
            read: read,
            line: String::new(),
            line_pos: 0,
        }
    }

    pub fn read_next_csv(&mut self) -> Result<String, ReadErr> {
        fn is_newline(s: &str, p: usize) -> bool {
            s[p..].chars().next().map(|c| c == '\r' || c == '\n').unwrap_or(false)
        }
        loop {
            if self.line.len() <= self.line_pos || is_newline(&self.line, self.line_pos) {
                self.line.clear();
                let read = self.read.read_line(&mut self.line)?;
                self.line_pos = 0;
                if read == 0 {
                    return Ok(String::new());
                }
                continue;
            }
            let entry_end = self.line[self.line_pos..].char_indices()
                        .skip_while(|&(_, c)| c.is_whitespace())
                        .find(|&(_, c)| c.is_whitespace() || c == ',');
            if let Some((end, c)) = entry_end {
                let next_pos = self.line_pos + end;
                let rv = self.line[self.line_pos..next_pos].to_owned();
                self.line_pos = next_pos + if c == ',' { 1 } else { 0 };
                return Ok(rv);
            } else {
                let prev_pos = self.line_pos;
                self.line_pos = self.line.len();
                return Ok(self.line[prev_pos..].to_owned());
            }
        }
    }

    pub fn read_rest_string(&mut self) -> Result<String, ReadErr> {
        if self.line.len() <= self.line_pos {
            self.line.clear();
            let read = self.read.read_line(&mut self.line)?;
            self.line_pos = 0;
            if read == 0 {
                return Ok(String::new());
            }
        }
        let last = self.line.rfind(|c| c == '\r' || c == '\n').unwrap_or(self.line.len());
        let rest = self.line[self.line_pos..last].to_owned();
        self.line_pos = self.line.len();
        Ok(rest)
    }

    pub fn read_value<T: FortranRead>(&mut self, val: &mut T) -> Result<bool, ReadErr> {
        val.fortran_read_default(self)
    }
}
