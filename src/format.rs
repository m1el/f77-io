//! Fortran format parser
//!
//! # Usage
//!
//! ```
//! use f77_io::format::{parse_format};
//! let fmt = parse_format("('hello world'/, I16)").unwrap();
//! assert_eq!(fmt.to_string(), "('hello world'/I16)");
//!
//! use f77_io::format::FormatNode::{Group, Literal, NewLine, Int};
//! assert_eq!(fmt, Group(vec![
//!     Literal("hello world".to_string()),
//!     NewLine, Int(Some(16), None)]));
//! ```
//!

use ::std::fmt::{Write};

#[derive(Debug, Clone, PartialEq)]
pub enum RealFormat {
    F,
    E,
    D,
    G,
}

impl From<char> for RealFormat {
    fn from(src: char) -> RealFormat {
        use self::RealFormat::*;
        match src {
            'F' | 'f' => F,
            'E' | 'e' => E,
            'D' | 'd' => D,
            'G' | 'g' => G,
            _ => panic!("invalid real format: {}", src),
        }
    }
}

impl<'a> Into<char> for &'a RealFormat {
    fn into(self) -> char {
        use self::RealFormat::*;
        match *self {
            F => 'F',
            E => 'E',
            D => 'D',
            G => 'G',
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlankType {
    BZ,
    BN,
    B,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TabType {
    T,
    TL,
    TR,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatNode {
    NewLine,
    SkipChar,
    SuppressNewLine,
    RemainingChars,
    Terminate,
    BlankControl(BlankType),
    AbsColumn(usize),
    RelColumn(isize),
    Radix(usize),
    Scale(isize),

    Literal(String),

    Str(Option<usize>),
    Bool(Option<usize>),
    Int(Option<usize>, Option<usize>),
    Oct(Option<usize>, Option<usize>),
    Hex(Option<usize>, Option<usize>),
    Real(RealFormat, Option<usize>, Option<usize>, Option<usize>),

    Group(Vec<FormatNode>),
    Repeat(usize, Box<FormatNode>),
}

impl FormatNode {
    pub fn write_string<W>(&self, out: &mut W) -> Result<(), ::std::fmt::Error>
        where W: Write
    {
        use self::FormatNode::*;
        match *self {
            NewLine => out.write_char('/'),
            SkipChar => out.write_char('X'),
            SuppressNewLine => out.write_char('$'),
            Terminate => out.write_char(':'),
            RemainingChars => out.write_char('Q'),
            BlankControl(BlankType::B) => out.write_char('B'),
            BlankControl(BlankType::BZ) => out.write_str("BZ"),
            BlankControl(BlankType::BN) => out.write_str("BN"),
            AbsColumn(w) => write!(out, "T{}", w),
            RelColumn(w) => {
                if w >= 0 { write!(out, "TR{}", w) }
                else { write!(out, "TL{}", -w) }
            },
            Radix(x) => write!(out, "{}R", x),
            Scale(x) => write!(out, "{}P", x),
            Literal(ref x) => {
                // TODO: newlines, non-alphanumerics?
                let escaped = x.split('\'').collect::<Vec<&str>>().join("''");
                write!(out, "'{}'", escaped)
            },
            Str(ow) => {
                match ow {
                    None => write!(out, "A"),
                    Some(w) => write!(out, "A{}", w),
                }
            },
            Bool(ow) => {
                match ow {
                    None => write!(out, "L"),
                    Some(w) => write!(out, "L{}", w),
                }
            },
            Int(ow, od) => {
                match (ow, od) {
                    (None, _) => write!(out, "I"),
                    (Some(w), None) => write!(out, "I{}", w),
                    (Some(w), Some(d)) => write!(out, "I{}.{}", w, d),
                }
            },
            Oct(ow, od) => {
                match (ow, od) {
                    (None, _) => write!(out, "O"),
                    (Some(w), None) => write!(out, "O{}", w),
                    (Some(w), Some(d)) => write!(out, "O{}.{}", w, d),
                }
            },
            Hex(ow, od) => {
                match (ow, od) {
                    (None, _) => write!(out, "Z"),
                    (Some(w), None) => write!(out, "Z{}", w),
                    (Some(w), Some(d)) => write!(out, "Z{}.{}", w, d),
                }
            },
            Real(ref f, ow, od, oe) => {
                let c: char = f.into();
                match (ow, od, oe) {
                    (None, _, _) => write!(out, "{}", c),
                    (Some(w), None, _) => write!(out, "{}{}", c, w),
                    (Some(w), Some(d), None) => write!(out, "{}{}.{}", c, w, d),
                    (Some(w), Some(d), Some(e)) => write!(out, "{}{}.{}E{}", c, w, d, e),
                }
            },
            Group(ref v) => {
                try!(out.write_char('('));
                let mut skip_comma = true;
                let mut was_slash = false;
                let mut it = v.iter();
                loop {
                    let node = match it.next() {
                        Some(x) => x,
                        None => break,
                    };

                    if *node == NewLine {
                        skip_comma = true;
                    }

                    if !skip_comma && !was_slash {
                        try!(out.write_str(", "));
                    }

                    skip_comma = false;
                    was_slash = *node == NewLine;
                    try!(node.write_string(out));
                }

                out.write_char(')')
            },
            Repeat(n, ref b) => {
                try!(write!(out, "{}", n));
                b.write_string(out)
            },
        }
    }
}

impl ToString for FormatNode {
    fn to_string(&self) -> String {
        let mut rv = String::new();
        self.write_string(&mut rv).unwrap();
        rv
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEOF(usize),
    ExpectedNumber(usize),
    ExpectedNonZero(usize),
    ExpectedParen(usize),
    ExpectedComma(usize),
    ExpectedScaleControl(usize),
    NumberTooBig(usize),
    RepeatingDollar(usize),
    RepeatingColon(usize),
    RepeatingStr(usize),
    RepeatingBlankControl(usize),
    RepeatingTab(usize),
    RepeatingQ(usize),
    ExtraComma(usize),
    MissingScale(usize),
    UnexpectedChar(usize, char),
    MissingRadix(usize),
    RadixOutOfRange(usize, usize),
}

use ::std::iter::{Peekable};
use ::std::str::{Chars};

pub fn parse_format(source: &str) -> Result<FormatNode, ParseError> {
    let mut it = source.chars().peekable();
    FormatParser::new(&mut it).parse()
}

pub struct FormatParser<'a> {
    it: &'a mut Peekable<Chars<'a>>,
    pos: usize,
}

impl<'a> FormatParser<'a> {
    pub fn new(it: &'a mut Peekable<Chars<'a>>) -> FormatParser<'a> {
        FormatParser {
            it: it,
            pos: 0,
        }
    }

    #[inline(always)]
    fn peek(&mut self) -> Option<char> {
        self.it.peek().map(|v| *v)
    }

    #[inline(always)]
    fn next(&mut self) -> Option<char> {
        self.pos += 1;
        self.it.next()
    }

    fn yield_digits(&mut self) -> Result<Option<usize>, ParseError> {
        let mut s = String::new();

        let start = self.pos;
        while self.peek().map(|c| '0' <= c && c <= '9') == Some(true) {
            s.push(self.next().unwrap());
        }

        if s.len() != 0 {
            match s.parse::<usize>() {
                Ok(x) => Ok(Some(x)),
                Err(_) => Err(ParseError::NumberTooBig(start)),
            }
        } else {
            Ok(None)
        }
    }

    fn yield_whitespace(&mut self) {
        // TODO: use char::is_whitespace?
        const WHITESPACE: &'static str = " \t\r\n";
        while self.peek().map(|c| WHITESPACE.contains(c)) == Some(true) {
            let _ = self.next();
        }
    }

    fn yield_string(&mut self, dst: &mut String, e: char) -> Result<(), ParseError> {
        loop {
            match self.next() {
                Some(c) => {
                    if c == e {
                        return Ok(());
                    }
                    dst.push(c);
                }
                None => return Err(ParseError::UnexpectedEOF(self.pos)),
            }
        }
    }

    fn yield_int_format(&mut self) -> Result<(Option<usize>, Option<usize>), ParseError> {
        use self::ParseError::*;

        self.yield_whitespace();
        let w = try!(self.yield_digits());
        let mut d = None;
        if w == Some(0) {
            return Err(ExpectedNonZero(self.pos));
        }
        if w.is_some() && self.peek() == Some('.') {
            let _ = self.next();
            self.yield_whitespace();
            d = try!(self.yield_digits());
            if !d.is_some() {
                return Err(ExpectedNumber(self.pos));
            }
        }
        return Ok((w, d));
    }

    // Here be dragons
    // TODO: add tokenizer layer to simplify the parser?
    // TODO: consider another approach for the parser?
    pub fn parse(&mut self) -> Result<FormatNode, ParseError> {
        use self::ParseError::*;
        use self::FormatNode::*;

        #[inline(always)]
        fn mk_repeating(repeat: Option<usize>, token: FormatNode) -> FormatNode {
            match repeat {
                Some(x) => Repeat(x, Box::new(token)),
                None => token,
            }
        }

        let mut result = vec![];

        self.yield_whitespace();

        if Some('(') != self.next() {
            return Err(ExpectedParen(self.pos));
        }

        let mut was_comma = false;
        let mut was_slash = false;
        loop {
            self.yield_whitespace();
            let p = match self.peek() {
                Some(x) => x,
                None => break,
            };

            if p != ')' && p != '/' && result.len() != 0 && !(was_comma||was_slash) {
                return Err(ExpectedComma(self.pos));
            }

            // Scale Control is the only source of negative value prefixes
            if p == '-' {
                let _ = self.next();
                let scale = match try!(self.yield_digits()) {
                    Some(x) => -(x as isize),
                    None => return Err(ExpectedNumber(self.pos)),
                };
                self.yield_whitespace();
                match self.next() {
                    Some('P') | Some('p') => {
                        result.push(Scale(scale));
                        continue;
                    },
                    Some(_) | None => {
                        return Err(ExpectedScaleControl(self.pos));
                    },
                }
            }

            let repeat = try!(self.yield_digits());
            self.yield_whitespace();

            if self.peek() == Some('(') {
                match self.parse() {
                    Ok(Group(v)) => {
                        result.push(mk_repeating(repeat, Group(v)));
                    },
                    Ok(_) => unreachable!(),
                    Err(x) => return Err(x),
                };

                self.yield_whitespace();

                if self.peek() == Some(',') {
                    was_comma = true;
                    let _ = self.next();
                } else {
                    was_comma = false;
                }

                continue;
            }

            let c = match self.next() {
                Some(x) => x,
                None => break,
            };

            was_slash = false;

            match c {
                ')' => {
                    if was_comma {
                        return Err(ExtraComma(self.pos));
                    }
                    return Ok(Group(result));
                },
                '/' => {
                    was_slash = true;
                    result.push(mk_repeating(repeat, NewLine));
                },
                '$' => {
                    if repeat.is_some() {
                        return Err(RepeatingDollar(self.pos));
                    }
                    result.push(SuppressNewLine);
                },
                ':' => {
                    if repeat.is_some() {
                        return Err(RepeatingColon(self.pos));
                    }
                    result.push(Terminate);
                },
                'X' => {
                    result.push(mk_repeating(repeat, SkipChar));
                },
                'Q' => {
                    if repeat.is_some() {
                        return Err(RepeatingQ(self.pos));
                    }
                    result.push(RemainingChars);
                },
                'I' | 'i' => {
                    let (w, d) = try!(self.yield_int_format());
                    result.push(mk_repeating(repeat, Int(w, d)));
                },
                'Z' | 'z' => {
                    let (w, d) = try!(self.yield_int_format());
                    result.push(mk_repeating(repeat, Hex(w, d)));
                },
                'O' | 'o' => {
                    let (w, d) = try!(self.yield_int_format());
                    result.push(mk_repeating(repeat, Oct(w, d)));
                },
                'L' | 'l' => {
                    self.yield_whitespace();
                    let w = try!(self.yield_digits());
                    if w == Some(0) {
                        return Err(ExpectedNonZero(self.pos));
                    }
                    result.push(mk_repeating(repeat, Bool(w)));
                },
                'A' | 'a' => {
                    self.yield_whitespace();
                    let w = try!(self.yield_digits());
                    if w == Some(0) {
                        return Err(ExpectedNonZero(self.pos));
                    }
                    result.push(mk_repeating(repeat, Str(w)));
                },
                '"' | '\'' => {
                    if repeat.is_some() {
                        return Err(RepeatingStr(self.pos));
                    }
                    let mut s = String::new();
                    let e = c;
                    loop {
                        // read quoted
                        try!(self.yield_string(&mut s, e));
                        // if quote repeats, it's escaped
                        if self.peek() == Some(e) {
                            s.push(e);
                            let _ = self.next();
                        } else {
                            break;
                        }
                    }
                    result.push(Literal(s));
                },
                'F' | 'f' | 'E' | 'e' | 'D' | 'd' | 'G' | 'g' => {
                    self.yield_whitespace();
                    let w = try!(self.yield_digits());
                    let mut d = None;
                    let mut e = None;
                    if w == Some(0) {
                        return Err(ExpectedNonZero(self.pos));
                    }
                    if w.is_some() && self.peek() == Some('.') {
                        let _ = self.next();
                        self.yield_whitespace();
                        d = try!(self.yield_digits());
                        if !d.is_some() {
                            return Err(ExpectedNumber(self.pos));
                        }
                    }
                    if d.is_some() && self.peek().map(|c| c == '.' || c == 'e' || c == 'E').unwrap_or(false) {
                        let _ = self.next();
                        self.yield_whitespace();
                        e = try!(self.yield_digits());
                        if e == Some(0) {
                            return Err(ExpectedNonZero(self.pos));
                        }
                        if !e.is_some() {
                            return Err(ExpectedNumber(self.pos));
                        }
                    }
                    result.push(mk_repeating(repeat, Real(RealFormat::from(c), w, d, e)));
                },
                'P' => {
                    match repeat {
                        Some(r) => result.push(Scale(r as isize)),
                        None => return Err(MissingScale(self.pos)),
                    }
                },
                'R' => {
                    match repeat {
                        Some(r) => {
                            if 2 <= r && r <= 36 {
                                result.push(Radix(r));
                            } else {
                                return Err(RadixOutOfRange(self.pos, r));
                            }
                        }
                        None => return Err(MissingRadix(self.pos)),
                    }
                },
                'B' => {
                    if repeat.is_some() {
                        return Err(RepeatingBlankControl(self.pos));
                    }

                    let blank_type = match self.peek() {
                        Some('N') => {
                            let _ = self.next();
                            BlankType::BN
                        },
                        Some('Z') => {
                            let _ = self.next();
                            BlankType::BZ
                        },
                        _ => BlankType::B,
                    };

                    result.push(BlankControl(blank_type));
                },
                'T' => {
                    if repeat.is_some() {
                        return Err(RepeatingTab(self.pos));
                    }

                    let tab_type = match self.peek() {
                        Some('L') => {
                            let _ = self.next();
                            TabType::TL
                        },
                        Some('R') => {
                            let _ = self.next();
                            TabType::TR
                        },
                        _ => TabType::T,
                    };

                    self.yield_whitespace();
                    let c = match try!(self.yield_digits()) {
                        Some(x) => x,
                        None => return Err(ExpectedNumber(self.pos)),
                    };

                    let node = match tab_type {
                        // T n
                        TabType::T => AbsColumn(c),
                        // TR n
                        TabType::TR => RelColumn(c as isize),
                        // TL n
                        TabType::TL => RelColumn(-(c as isize)),
                    };
                    result.push(node);
                },
                _ => {
                    return Err(UnexpectedChar(self.pos, c))
                },
            }

            self.yield_whitespace();

            if self.peek() == Some(',') {
                was_comma = true;
                let _ = self.next();
            } else {
                was_comma = false;
            }
        }

        return Err(UnexpectedEOF(self.pos));
    }
}

// TODO: add tests for the format parser
// test for REAL and INT format parsing/stringify
#[cfg(test)]
mod tests {
    use ::format::FormatNode::*;
    use ::format::parse_format;

    #[test]
    fn empty() {
        assert_eq!(parse_format("()").unwrap(), Group(vec![]));
    }

    #[test]
    fn err_eof() {
        assert!(parse_format("(").is_err());
    }

    #[test]
    fn err_paren() {
        assert!(parse_format("/").is_err());
    }

    #[test]
    fn unescape_quot() {
        let src = "(' '' ')";
        let parsed = Group(vec![Literal(" ' ".to_string())]);
        assert_eq!(parse_format(src).unwrap(), parsed);
    }

    #[test]
    fn escape_quot() {
        let parsed = Literal(" ' ".to_string());
        let src = "' '' '";
        assert_eq!(parsed.to_string(), src);
    }

    #[test]
    fn some_header() {
        let src = concat!(
            "('1'/1X,125('*')/1X,125('*')/1X,50('*'),25X,50('*')/1X,",
            "50('*'),10X,'FOBAR',10X,50('*')/1X,50('*'),25X,50('*')",
            "/1X,125('*')/1X,125('*')////)");

        let parsed = Group(vec![
           Literal("1".to_string()), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(125, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(125, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))),
           Repeat(25, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))),
           Repeat(10, Box::new(SkipChar)), Literal("FOBAR".to_string()),
           Repeat(10, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))),
           Repeat(25, Box::new(SkipChar)), Repeat(50, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(125, Box::new(Group(vec![Literal("*".to_string())]))), NewLine,
           Repeat(1, Box::new(SkipChar)), Repeat(125, Box::new(Group(vec![Literal("*".to_string())]))),
           NewLine, NewLine, NewLine, NewLine]);

        assert_eq!(parse_format(src).unwrap(), parsed);
    }

    // NOTE: this test is sensitive to whitespace.
    // do we want to change this behavior?
    #[test]
    fn some_header_str() {
        let src = concat!(
            "('1'/1X,125('*')/1X,125('*')/1X,50('*'),25X,50('*')/1X,",
            "50('*'),10X,'FOBAR',10X,50('*')/1X,50('*'),25X,50('*')",
            "/1X,125('*')/1X,125('*')////)");
        let expected = concat!(
            "('1'/1X, 125('*')/1X, 125('*')/1X, 50('*'), 25X, 50('*')/1X, ",
            "50('*'), 10X, 'FOBAR', 10X, 50('*')/1X, 50('*'), ",
            "25X, 50('*')/1X, 125('*')/1X, 125('*')////)");

        assert_eq!(parse_format(src).unwrap().to_string(), expected);
    }
}
