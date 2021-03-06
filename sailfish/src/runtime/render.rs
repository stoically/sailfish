use std::borrow::Cow;
use std::cell::{Ref, RefMut};
use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
    NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize, Wrapping,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

use super::buffer::Buffer;
use super::{escape, RenderError};

/// types which can be rendered inside buffer block (`<%= %>`)
///
/// If you want to render the custom data, you must implement this trait and specify
/// the behaviour.
///
/// # Examples
///
/// ```
/// use sailfish::runtime::{Buffer, Render, RenderError};
///
/// struct MyU64(u64);
///
/// impl Render for MyU64 {
///     #[inline]
///     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
///         self.0.render(b)
///     }
/// }
/// ```
pub trait Render {
    /// render to `Buffer` without escaping
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError>;

    /// render to `Buffer` with HTML escaping
    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        let mut tmp = Buffer::new();
        self.render(&mut tmp)?;
        escape::escape_to_buf(tmp.as_str(), b);
        Ok(())
    }
}

// /// Autoref-based stable specialization
// ///
// /// Explanation can be found [here](https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md)
// impl<T: Display> Render for &T {
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         fmt::write(b, format_args!("{}", self))
//     }
//
//     fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         struct Wrapper<'a>(&'a mut Buffer);
//
//         impl<'a> fmt::Write for Wrapper<'a> {
//             #[inline]
//             fn push_str(&mut self, s: &str) -> Result<(), RenderError> {
//                 escape::escape_to_buf(s, self.0);
//                 Ok(())
//             }
//         }
//
//         fmt::write(&mut Wrapper(b), format_args!("{}", self))
//     }
// }

impl Render for String {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        b.push_str(&**self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(&**self, b);
        Ok(())
    }
}

impl Render for &str {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        b.push_str(*self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(*self, b);
        Ok(())
    }
}

impl Render for char {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        b.push(*self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        match *self {
            '\"' => b.push_str("&quot;"),
            '&' => b.push_str("&amp;"),
            '<' => b.push_str("&lt;"),
            '>' => b.push_str("&gt;"),
            '\'' => b.push_str("&#039;"),
            _ => b.push(*self),
        }
        Ok(())
    }
}

impl Render for PathBuf {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        // TODO: speed up on Windows using OsStrExt
        b.push_str(&*self.to_string_lossy());
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(&*self.to_string_lossy(), b);
        Ok(())
    }
}

impl Render for Path {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        // TODO: speed up on Windows using OsStrExt
        b.push_str(&*self.to_string_lossy());
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(&*self.to_string_lossy(), b);
        Ok(())
    }
}

// impl Render for [u8] {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(self);
//         Ok(())
//     }
// }
//
// impl<'a> Render for &'a [u8] {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(self);
//         Ok(())
//     }
// }
//
// impl Render for Vec<u8> {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(&**self);
//         Ok(())
//     }
// }

impl Render for bool {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        let s = if *self { "true" } else { "false" };
        b.push_str(s);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        self.render(b)
    }
}

macro_rules! render_int {
    ($($int:ty),*) => {
        $(
            impl Render for $int {
                #[cfg_attr(feature = "perf-inline", inline)]
                fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    use itoap::Integer;

                    b.reserve(Self::MAX_LEN);

                    unsafe {
                        let ptr = b.as_mut_ptr().add(b.len());
                        let l = itoap::write_to_ptr(ptr, *self);
                        b.advance(l);
                    }
                    debug_assert!(b.len() <= b.capacity());
                    Ok(())
                }

                #[inline]
                fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    // push_str without escape
                    self.render(b)
                }
            }
        )*
    }
}

render_int!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);

impl Render for f32 {
    #[cfg_attr(feature = "perf-inline", inline)]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        if likely!(self.is_finite()) {
            unsafe {
                b.reserve(16);
                let ptr = b.as_mut_ptr().add(b.len());
                let l = ryu::raw::format32(*self, ptr);
                b.advance(l);
                debug_assert!(b.len() <= b.capacity());
            }
        } else if self.is_nan() {
            b.push_str("NaN");
        } else if *self > 0.0 {
            b.push_str("inf");
        } else {
            b.push_str("-inf");
        }

        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        // escape string
        self.render(b)
    }
}

impl Render for f64 {
    #[cfg_attr(feature = "perf-inline", inline)]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        if likely!(self.is_finite()) {
            unsafe {
                b.reserve(24);
                let ptr = b.as_mut_ptr().add(b.len());
                let l = ryu::raw::format64(*self, ptr);
                b.advance(l);
                debug_assert!(b.len() <= b.capacity());
            }
        } else if self.is_nan() {
            b.push_str("NaN");
        } else if *self > 0.0 {
            b.push_str("inf");
        } else {
            b.push_str("-inf");
        }

        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        // escape string
        self.render(b)
    }
}

macro_rules! render_deref {
    (
        $(#[doc = $doc:tt])*
        [$($bounds:tt)+] $($desc:tt)+
    ) => {
        $(#[doc = $doc])*
        impl <$($bounds)+> Render for $($desc)+ {
            #[inline]
            fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
                (**self).render(b)
            }

            #[inline]
            fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
                (**self).render_escaped(b)
            }
        }
    };
}

render_deref!(['a, T: Render + ?Sized] &'a T);
render_deref!(['a, T: Render + ?Sized] &'a mut T);
render_deref!([T: Render + ?Sized] Box<T>);
render_deref!([T: Render + ?Sized] Rc<T>);
render_deref!([T: Render + ?Sized] Arc<T>);
render_deref!(['a, T: Render + ToOwned + ?Sized] Cow<'a, T>);
render_deref!(['a, T: Render + ?Sized] Ref<'a, T>);
render_deref!(['a, T: Render + ?Sized] RefMut<'a, T>);
render_deref!(['a, T: Render + ?Sized] MutexGuard<'a, T>);
render_deref!(['a, T: Render + ?Sized] RwLockReadGuard<'a, T>);
render_deref!(['a, T: Render + ?Sized] RwLockWriteGuard<'a, T>);

macro_rules! render_nonzero {
    ($($type:ty,)*) => {
        $(
            impl Render for $type {
                #[inline]
                fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    self.get().render(b)
                }

                #[inline]
                fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    self.get().render_escaped(b)
                }
            }
        )*
    }
}

render_nonzero!(
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroIsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroUsize,
);

impl<T: Render> Render for Wrapping<T> {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        self.0.render(b)
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        self.0.render_escaped(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_coercion() {
        let mut b = Buffer::new();
        Render::render(&1, &mut b).unwrap();
        Render::render(&&1, &mut b).unwrap();
        Render::render(&&&1, &mut b).unwrap();
        Render::render(&&&&1, &mut b).unwrap();
        assert_eq!(b.as_str(), "1111");
        b.clear();

        Render::render(&true, &mut b).unwrap();
        Render::render(&&false, &mut b).unwrap();
        Render::render(&&&true, &mut b).unwrap();
        Render::render(&&&&false, &mut b).unwrap();
        assert_eq!(b.as_str(), "truefalsetruefalse");
        b.clear();

        let s = "apple";
        Render::render_escaped(&s, &mut b).unwrap();
        Render::render_escaped(&s, &mut b).unwrap();
        Render::render_escaped(&&s, &mut b).unwrap();
        Render::render_escaped(&&&s, &mut b).unwrap();
        Render::render_escaped(&&&&s, &mut b).unwrap();
        assert_eq!(b.as_str(), "appleappleappleappleapple");
        b.clear();

        Render::render_escaped(&'c', &mut b).unwrap();
        Render::render_escaped(&&'<', &mut b).unwrap();
        Render::render_escaped(&&&'&', &mut b).unwrap();
        Render::render_escaped(&&&&' ', &mut b).unwrap();
        assert_eq!(b.as_str(), "c&lt;&amp; ");
        b.clear();
    }

    #[test]
    fn deref_coercion() {
        use std::path::PathBuf;
        use std::rc::Rc;

        let mut b = Buffer::new();
        Render::render(&String::from("a"), &mut b).unwrap();
        Render::render(&&PathBuf::from("b"), &mut b).unwrap();
        Render::render_escaped(&Rc::new(4u32), &mut b).unwrap();
        Render::render_escaped(&Rc::new(2.3f32), &mut b).unwrap();

        assert_eq!(b.as_str(), "ab42.3");
    }

    #[test]
    fn float() {
        let mut b = Buffer::new();

        Render::render_escaped(&0.0f64, &mut b).unwrap();
        Render::render_escaped(&std::f64::INFINITY, &mut b).unwrap();
        Render::render_escaped(&std::f64::NEG_INFINITY, &mut b).unwrap();
        Render::render_escaped(&std::f64::NAN, &mut b).unwrap();
        assert_eq!(b.as_str(), "0.0inf-infNaN");
        b.clear();

        Render::render_escaped(&0.0f32, &mut b).unwrap();
        Render::render_escaped(&std::f32::INFINITY, &mut b).unwrap();
        Render::render_escaped(&std::f32::NEG_INFINITY, &mut b).unwrap();
        Render::render_escaped(&std::f32::NAN, &mut b).unwrap();
        assert_eq!(b.as_str(), "0.0inf-infNaN");
    }
}
