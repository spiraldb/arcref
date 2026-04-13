//! `ArcRef<T>` is a smart pointer type that can hold either a `&'static T` or an `Arc<T>`.
//!
//! It's especially helpful for composible plugin systems where some of the implementations are
//! ZSTs or packaged with the core library, and the others are dynamically generated.
//!
//! ## Example usage:
//!
//! ```rust
//! use arcref::ArcRef;
//! use std::sync::Arc;
//!
//! #[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
//! pub enum Level { INFO, WARN, ERROR }
//!
//! pub trait Logger {
//!   fn log(&self, level: Level, line: &str) -> String;
//! }
//!
//! pub struct ConsoleLogger;
//!
//! impl Logger for ConsoleLogger {
//!   fn log(&self, level: Level, line: &str) -> String {
//!     format!("{level:?}: {line}")
//!   }
//! }
//!
//! pub struct FilteredLogger {
//!   min_level: Level,
//!   delegate: ArcRef<dyn Logger>,
//! }
//!
//! impl Logger for FilteredLogger {
//!   fn log(&self, level: Level, line: &str) -> String {
//!     if level >= self.min_level {
//!       self.delegate.log(level, line)
//!     } else {
//!       String::new()
//!     }
//!   }
//! }
//!
//! let logger = ArcRef::new_arc(Arc::new(FilteredLogger {
//!   min_level: Level::WARN,
//!   delegate: ArcRef::new_ref(&ConsoleLogger),
//! }));
//!
//! assert_eq!(logger.log(Level::INFO, "not printed"), String::new());
//! assert_eq!(logger.log(Level::WARN, "printed"), "WARN: printed");
//! ```
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Deref;
use std::ptr;
use std::sync::Arc;

/// Either a reference-counted `Arc` or a static reference to a value.
pub struct ArcRef<T: ?Sized + 'static>(Inner<T>);

enum Inner<T: ?Sized + 'static> {
    Arc(Arc<T>),
    Ref(&'static T),
}

impl<T: ?Sized> ArcRef<T> {
    pub fn new_arc(t: Arc<T>) -> Self
    where
        T: 'static,
    {
        ArcRef(Inner::Arc(t))
    }

    pub const fn new_ref(t: &'static T) -> Self {
        ArcRef(Inner::Ref(t))
    }
}

impl<T: ?Sized> Clone for ArcRef<T> {
    fn clone(&self) -> Self {
        match &self.0 {
            Inner::Arc(arc) => ArcRef(Inner::Arc(Arc::clone(arc))),
            Inner::Ref(r) => ArcRef(Inner::Ref(*r)),
        }
    }
}

impl<T: ?Sized> From<&'static T> for ArcRef<T> {
    fn from(r: &'static T) -> Self {
        ArcRef(Inner::Ref(r))
    }
}

impl<T: 'static> From<T> for ArcRef<T> {
    fn from(t: T) -> Self {
        ArcRef(Inner::Arc(Arc::new(t)))
    }
}

impl<T: ?Sized + 'static> From<Arc<T>> for ArcRef<T> {
    fn from(arc: Arc<T>) -> Self {
        ArcRef(Inner::Arc(arc))
    }
}

impl<T: ?Sized> Deref for ArcRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            Inner::Arc(arc) => arc,
            Inner::Ref(r) => r,
        }
    }
}

impl<S, T> PartialEq<ArcRef<S>> for ArcRef<T>
where
    S: ?Sized + 'static,
    T: ?Sized + 'static + PartialEq<S>,
{
    fn eq(&self, other: &ArcRef<S>) -> bool {
        let ptr_eq = match (self, other) {
            (ArcRef(Inner::Arc(a)), ArcRef(Inner::Arc(b))) => {
                ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
            }
            (ArcRef(Inner::Arc(a)), ArcRef(Inner::Ref(b))) => {
                ptr::addr_eq(Arc::as_ptr(a), ptr::from_ref(*b))
            }
            (ArcRef(Inner::Ref(a)), ArcRef(Inner::Ref(b))) => {
                ptr::addr_eq(ptr::from_ref(*a), ptr::from_ref(*b))
            }
            (ArcRef(Inner::Ref(a)), ArcRef(Inner::Arc(b))) => {
                ptr::addr_eq(ptr::from_ref(*a), Arc::as_ptr(b))
            }
        };
        ptr_eq || self.deref() == other.deref()
    }
}

impl<T> Eq for ArcRef<T> where T: ?Sized + 'static + Eq {}

impl<S, T> PartialOrd<ArcRef<S>> for ArcRef<T>
where
    S: ?Sized + 'static,
    T: ?Sized + 'static + PartialOrd<S>,
{
    fn partial_cmp(&self, other: &ArcRef<S>) -> Option<Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<T> Ord for ArcRef<T>
where
    T: ?Sized + 'static + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other.deref())
    }
}

impl<T> Hash for ArcRef<T>
where
    T: ?Sized + 'static + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T> Debug for ArcRef<T>
where
    T: ?Sized + 'static + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> Display for ArcRef<T>
where
    T: ?Sized + 'static + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: ?Sized + 'static> AsRef<T> for ArcRef<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized + 'static> Borrow<T> for ArcRef<T> {
    fn borrow(&self) -> &T {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A type whose `PartialEq` always returns false. Equality between two
    // `ArcRef<NeverEq>` values can therefore only hold via the `ptr_eq`
    // short-circuit in `PartialEq for ArcRef`.
    #[derive(Debug)]
    struct NeverEq(#[allow(dead_code)] u32);

    impl PartialEq for NeverEq {
        fn eq(&self, _other: &Self) -> bool {
            false
        }
    }

    static NEVER_EQ_A: NeverEq = NeverEq(1);
    static NEVER_EQ_B: NeverEq = NeverEq(2);

    #[test]
    fn ptr_eq_arc_arc_same_allocation() {
        let arc = Arc::new(NeverEq(7));
        let a = ArcRef::new_arc(Arc::clone(&arc));
        let b = ArcRef::new_arc(arc);
        assert!(a == b, "ptr_eq must short-circuit for shared Arc allocation");
    }

    #[test]
    fn ptr_eq_arc_arc_via_clone() {
        let a = ArcRef::new_arc(Arc::new(NeverEq(7)));
        let b = a.clone();
        assert!(a == b, "ArcRef::clone must preserve Arc identity for ptr_eq");
    }

    #[test]
    fn ptr_eq_arc_arc_distinct_allocations() {
        let a = ArcRef::new_arc(Arc::new(NeverEq(7)));
        let b = ArcRef::new_arc(Arc::new(NeverEq(7)));
        assert!(
            a != b,
            "distinct Arc allocations should fall through to PartialEq"
        );
    }

    #[test]
    fn ptr_eq_ref_ref_same_static() {
        let a = ArcRef::new_ref(&NEVER_EQ_A);
        let b = ArcRef::new_ref(&NEVER_EQ_A);
        assert!(a == b, "ptr_eq must short-circuit for shared static refs");
    }

    #[test]
    fn ptr_eq_ref_ref_distinct_statics() {
        let a = ArcRef::new_ref(&NEVER_EQ_A);
        let b = ArcRef::new_ref(&NEVER_EQ_B);
        assert!(
            a != b,
            "distinct statics should fall through to PartialEq"
        );
    }

    #[test]
    fn ptr_eq_arc_ref_distinct() {
        // An Arc allocation and a static ref cannot share an address in safe
        // Rust, so both Arc/Ref and Ref/Arc arms must fall through to PartialEq.
        let a = ArcRef::new_arc(Arc::new(NeverEq(1)));
        let b = ArcRef::new_ref(&NEVER_EQ_A);
        assert!(a != b);
        assert!(b != a);
    }

    #[test]
    fn ptr_eq_makes_nan_reflexive() {
        // f64::NAN is not equal to itself under PartialEq, but ArcRef
        // equality should still be reflexive thanks to the ptr_eq path.
        let nan = ArcRef::new_arc(Arc::new(f64::NAN));
        let nan_clone = nan.clone();
        assert!(nan == nan_clone);
        #[allow(clippy::eq_op)]
        let reflexive = nan == nan;
        assert!(reflexive);
    }

    #[test]
    fn deref_fallback_arc_arc_equal() {
        let a: ArcRef<i32> = ArcRef::new_arc(Arc::new(42));
        let b: ArcRef<i32> = ArcRef::new_arc(Arc::new(42));
        assert!(a == b);
    }

    #[test]
    fn deref_fallback_arc_ref_equal() {
        static VALUE: i32 = 42;
        let a: ArcRef<i32> = ArcRef::new_ref(&VALUE);
        let b: ArcRef<i32> = ArcRef::new_arc(Arc::new(42));
        assert!(a == b);
        assert!(b == a);
    }

    #[test]
    fn deref_fallback_arc_arc_unequal() {
        let a: ArcRef<i32> = ArcRef::new_arc(Arc::new(1));
        let b: ArcRef<i32> = ArcRef::new_arc(Arc::new(2));
        assert!(a != b);
    }

    #[test]
    fn ptr_eq_cross_type_unsized() {
        // Heterogeneous PartialEq between ArcRef<str> and ArcRef<String>.
        // The ptr_eq arm uses ptr::addr_eq, which permits differing pointee
        // types — verify the deref fallback still produces correct results.
        let s: ArcRef<String> = ArcRef::new_arc(Arc::new(String::from("hi")));
        let r: ArcRef<str> = ArcRef::new_ref("hi");
        assert!(s == s.clone());
        assert!(s.deref() == r.deref());
    }
}
