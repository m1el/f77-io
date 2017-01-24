//! Rust types to Fortran types association

// TODO: use tuples to represent complex numbers instead of using a library?
extern crate num_complex;
use self::num_complex::Complex;

macro_rules! impl_bool {
    ($x: ident) => {
        #[derive(Clone, Copy)]
        pub struct $x(bool);
        impl From<$x> for bool {
            fn from(x: $x) -> bool {
                x.0
            }
        }
    }
}
impl_bool!(Fbool2);
impl_bool!(Fbool4);
impl_bool!(Fbool8);

#[derive(Debug)]
pub enum FortranTag {
    Byte,
    Bool,
    Bool2,
    Bool4,
    Bool8,
    Int2,
    Int4,
    Int8,
    Uint2,
    Uint4,
    Uint8,
    Real4,
    Real8,
    Strin,
    Complex4,
    Complex8,
}

pub struct FortranType {
    pub tag: FortranTag,
    pub dim: Option<Vec<usize>>,
}

pub trait FortranAltType {
    fn fortran_tag() -> FortranTag;
    fn fortran_type() -> FortranType;
}

pub trait FortranAryType {
    fn fortran_tag() -> FortranTag;
    fn fortran_type(Vec<usize>) -> FortranType;
}

macro_rules! impl_primitive {
    ($tag: ident, $ty: ty) => {
        impl FortranAltType for $ty {
            fn fortran_tag() -> FortranTag {
                FortranTag::$tag
            }
            fn fortran_type() -> FortranType {
                FortranType {
                    tag: FortranTag::$tag,
                    dim: None,
                }
            }
        }
    }
}

macro_rules! impl_ary {
    ($tag: ident, $ty: ty) => {
        impl FortranAryType for Vec<$ty> {
            fn fortran_tag() -> FortranTag {
                type T = $ty;
                T::fortran_tag()
            }
            fn fortran_type(dim: Vec<usize>) -> FortranType {
                type T = $ty;
                FortranType {
                    tag: T::fortran_tag(),
                    dim: Some(dim),
                }
            }
        }
    }
}

impl_primitive!(Bool, bool);
impl_primitive!(Bool2, Fbool2);
impl_primitive!(Bool4, Fbool4);
impl_primitive!(Bool8, Fbool8);
impl_primitive!(Byte, i8);
impl_primitive!(Int2, i16);
impl_primitive!(Int4, i32);
impl_primitive!(Int8, i64);
impl_primitive!(Uint2, u16);
impl_primitive!(Uint4, u32);
impl_primitive!(Uint8, u64);
impl_primitive!(Real4, f32);
impl_primitive!(Real8, f64);
impl_primitive!(Strin, String);
impl_primitive!(Complex4, Complex<f32>);
impl_primitive!(Complex8, Complex<f64>);
impl_ary!(Bool, bool);
impl_ary!(Bool2, Fbool2);
impl_ary!(Bool4, Fbool4);
impl_ary!(Bool8, Fbool8);
impl_ary!(Byte, i8);
impl_ary!(Int2, i16);
impl_ary!(Int4, i32);
impl_ary!(Int8, i64);
impl_ary!(Uint2, u16);
impl_ary!(Uint4, u32);
impl_ary!(Uint8, u64);
impl_ary!(Real4, f32);
impl_ary!(Real8, f64);
// no array of strings
impl_ary!(Complex4, Complex<f32>);
impl_ary!(Complex8, Complex<f64>);
