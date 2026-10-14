//@ charon-args=--remove-associated-types=*
//! Two traits naming each other through their associated types' bounds, like midnight-curves'
//! `CurveExt` (`type AffineExt: CurveAffine<CurveExt = Self, ...>`) and `CurveAffine`
//! (`type CurveExt: CurveExt<AffineExt = Self, ...>`). The trait whose processing re-enters
//! itself used to keep all its associated types, so a bound like `A: Affine<Scalar = F>` stayed
//! a type constraint over projections; both traits should lift their own associated types.
pub trait Proj: Sized {
    type Scalar: Copy;
    type Base;
    type Affine: Affine<Scalar = Self::Scalar, Proj = Self>;

    fn scalar(&self) -> Self::Scalar;
}

pub trait Affine: Sized {
    type Scalar: Copy;
    type Base;
    type Proj: Proj<Scalar = Self::Scalar, Affine = Self>;

    fn get(&self) -> Self::Scalar;
}

pub fn scalar<F: Copy, A: Affine<Scalar = F>>(a: &A) -> F {
    a.get()
}
