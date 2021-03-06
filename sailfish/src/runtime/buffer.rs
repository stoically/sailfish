use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::fmt;
use std::mem::{align_of, ManuallyDrop};
use std::ops::{Add, AddAssign};
use std::ptr;

/// Buffer for rendered contents
///
/// This struct is quite simular to `String`, but some methods are
/// re-implemented for faster buffering.
pub struct Buffer {
    data: *mut u8,
    len: usize,
    capacity: usize,
}

impl Buffer {
    #[inline]
    pub const fn new() -> Buffer {
        Self {
            data: align_of::<u8>() as *mut u8, // dangling pointer
            len: 0,
            capacity: 0,
        }
    }

    #[cfg_attr(feature = "perf-inline", inline)]
    pub fn with_capacity(n: usize) -> Buffer {
        unsafe {
            if unlikely!(n == 0) {
                Self::new()
            } else {
                Self {
                    data: safe_alloc(n),
                    len: 0,
                    capacity: n,
                }
            }
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe {
            let bytes = std::slice::from_raw_parts(self.data, self.len);
            std::str::from_utf8_unchecked(bytes)
        }
    }

    #[inline]
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    #[doc(hidden)]
    pub unsafe fn _set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    /// Increase the length of buffer by `additional` bytes
    ///
    /// # Safety
    ///
    /// - `additional` must be less than or equal to `capacity() - len()`
    /// - The elements at `old_len..old_len + additional` must be initialized
    #[inline]
    pub unsafe fn advance(&mut self, additional: usize) {
        self.len += additional;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn reserve(&mut self, size: usize) {
        if size <= self.capacity.wrapping_sub(self.len) {
            return;
        }
        self.reserve_internal(size);
        debug_assert!(self.len + size <= self.capacity);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Converts a `Buffer` into a `String`.
    ///
    /// This consumes the `Buffer`, so we do not need to copy its contents.
    #[inline]
    pub fn into_string(self) -> String {
        debug_assert!(self.len <= self.capacity);
        let buf = ManuallyDrop::new(self);
        unsafe { String::from_raw_parts(buf.data, buf.len, buf.capacity) }
    }

    #[inline]
    pub fn push_str(&mut self, data: &str) {
        let size = data.len();
        if unlikely!(size > self.capacity.wrapping_sub(self.len)) {
            self.reserve_internal(size);
        }
        unsafe {
            let p = self.data.add(self.len);
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, size);
            self.len += size;
        }
        debug_assert!(self.len <= self.capacity);
    }

    #[inline]
    pub fn push(&mut self, data: char) {
        let mut buf = [0u8; 4];
        self.push_str(data.encode_utf8(&mut buf));
    }

    #[cfg_attr(feature = "perf-inline", inline)]
    #[cold]
    fn reserve_internal(&mut self, size: usize) {
        unsafe {
            let new_capacity = std::cmp::max(self.capacity * 2, self.capacity + size);
            debug_assert!(new_capacity > self.capacity);
            self.data = safe_realloc(self.data, self.capacity, new_capacity, size);
            self.capacity = new_capacity;
        }
        debug_assert!(!self.data.is_null());
        debug_assert!(self.len <= self.capacity);
    }
}

unsafe fn safe_alloc(capacity: usize) -> *mut u8 {
    assert!(capacity <= std::usize::MAX / 2, "capacity is too large");
    let layout = Layout::from_size_align_unchecked(capacity, 1);
    let data = alloc(layout);
    if data.is_null() {
        handle_alloc_error(layout);
    }

    data
}

#[cold]
unsafe fn safe_realloc(
    ptr: *mut u8,
    capacity: usize,
    new_capacity: usize,
    size: usize,
) -> *mut u8 {
    assert!(size <= std::usize::MAX / 2, "capacity is too large");
    assert!(new_capacity <= std::usize::MAX / 2, "capacity is too large");
    let data = if unlikely!(capacity == 0) {
        let new_layout = Layout::from_size_align_unchecked(new_capacity, 1);
        alloc(new_layout)
    } else {
        let old_layout = Layout::from_size_align_unchecked(capacity, 1);
        realloc(ptr, old_layout, new_capacity)
    };

    if data.is_null() {
        handle_alloc_error(Layout::from_size_align_unchecked(new_capacity, 1));
    }

    data
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        unsafe {
            if self.capacity == 0 {
                Self::new()
            } else {
                let buf = Self {
                    data: safe_alloc(self.len),
                    len: self.len,
                    capacity: self.len,
                };

                ptr::copy_nonoverlapping(self.data, buf.data, self.len);
                buf
            }
        }
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.capacity != 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.capacity, 1);
                dealloc(self.data, layout);
            }
        }
    }
}

impl fmt::Write for Buffer {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Buffer::push_str(self, s);
        Ok(())
    }
}

impl From<String> for Buffer {
    /// Shrink the data and pass raw pointer directory to buffer
    ///
    /// This operation is `O(1)`
    #[inline]
    fn from(other: String) -> Buffer {
        let bs = other.into_boxed_str();
        let data = unsafe { &mut *Box::into_raw(bs) };
        Buffer {
            data: data.as_mut_ptr(),
            len: data.len(),
            capacity: data.len(),
        }
    }
}

impl From<&str> for Buffer {
    #[inline]
    fn from(other: &str) -> Buffer {
        let mut buf = Buffer::with_capacity(other.len());
        unsafe {
            ptr::copy_nonoverlapping(other.as_ptr(), buf.as_mut_ptr(), other.len());
            buf.advance(other.len());
        }
        buf
    }
}

impl Add<&str> for Buffer {
    type Output = Buffer;

    #[inline]
    fn add(mut self, other: &str) -> Buffer {
        self.push_str(other);
        self
    }
}

impl AddAssign<&str> for Buffer {
    #[inline]
    fn add_assign(&mut self, other: &str) {
        self.push_str(other)
    }
}

impl Default for Buffer {
    #[inline]
    fn default() -> Buffer {
        Buffer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;

    #[test]
    fn test1() {
        let mut buffer = Buffer::new();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 0);

        buffer.push_str("apple");
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.capacity(), 5);

        buffer.push_str("pie");
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer.capacity(), 10);
    }

    #[test]
    fn test2() {
        let mut buffer = Buffer::with_capacity(1);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert!(buffer.capacity() >= 1);

        buffer += "pie";
        assert!(!buffer.is_empty());
    }

    #[test]
    fn string_conversion() {
        // from empty string
        let s = String::new();
        let mut buf = Buffer::from(s);
        assert_eq!(buf.as_str(), "");
        buf.push_str("abc");
        assert_eq!(buf.as_str(), "abc");

        // into non-empty string
        let mut s = buf.into_string();
        assert_eq!(s, "abc");

        s.push_str("defghijklmn");
        assert_eq!(s, "abcdefghijklmn");

        // from non-empty string
        let mut buf = Buffer::from(s);
        assert_eq!(buf.as_str(), "abcdefghijklmn");
        buf.clear();
        assert_eq!(buf.as_str(), "");

        // into empty string
        let buf = Buffer::default();
        let mut s = buf.into_string();
        assert_eq!(s, "");

        s.push_str("apple");
        assert_eq!(s, "apple");
    }

    #[test]
    fn clone() {
        use std::fmt::Write;

        let mut s1 = Buffer::with_capacity(0);
        let mut s2 = s1.clone();

        s1.push('a');
        s2.push_str("b");

        assert_eq!(s1.as_str(), "a");
        assert_eq!(s2.as_str(), "b");

        let mut s1 = Buffer::from("foo");
        let mut s2 = s1.clone();

        s1 = s1 + "bar";
        write!(s2, "baz").unwrap();

        assert_eq!(s1.as_str(), "foobar");
        assert_eq!(s2.as_str(), "foobaz");
    }
}
