use std::{mem, ptr};

/// A pair consisting of a raw pointer (`*const T`) and an integer value, packed so that it takes the size of a pointer.
///
/// It is implemented by packing the integer value in the low bits of the pointer that are known to be
/// zero because of alignment constraints.
///
/// The size of the value that can be stored alongside the pointer is 3 bits for most types, but ultimately depends on the minimum alignment of `T`:
/// for example, if `mem::align_of::<T>() == 16` then 4 bits are available to store the value.
///
/// # Notes
/// Pointers to zero-sized types do not have enough space to store any value, so it must be zero.
#[repr(transparent)]
#[derive(Debug)]
pub struct PointerValuePair<T: ?Sized> {
    pv: *const T,
}

impl<T: ?Sized> Copy for PointerValuePair<T> {}

impl<T: ?Sized> Clone for PointerValuePair<T> {
    fn clone(&self) -> Self {
        PointerValuePair { pv: self.pv }
    }
}

/// Returns a bitmask of the zero low bits of `*const T` pointers.
const fn align_bits<T>() -> usize {
    mem::align_of::<T>() - 1
}

impl<T> PointerValuePair<T> {
    /// Creates a new `PointerValuePair` from the given raw pointer and extra bits.
    ///
    /// # Panics
    ///
    /// Panics if the pointer type `*const T` does not have enough available low bits to store
    /// the value.
    pub fn new(ptr: *const T, value: usize) -> PointerValuePair<T> {
        let m = align_bits::<T>();
        assert!(
            value <= m,
            "not enough alignment bits ({}) to store the value ({})",
            Self::available_bits(),
            value
        );

        let mut repr = ptr as usize;
        repr |= value;

        PointerValuePair { pv: repr as *const T }
    }

    /// Returns the pointer.
    pub fn ptr(self) -> *const T {
        (self.pv as usize & !align_bits::<T>()) as *const T
    }

    /// Returns the value stored alongside the pointer.
    pub fn value(self) -> usize {
        self.pv as usize & align_bits::<T>()
    }

    /// Returns the number of bits available to store the value.
    pub const fn available_bits() -> u32 {
        align_bits::<T>().count_ones()
    }

    /// Returns the maximum (inclusive) integer value that can be stored in the pointer.
    pub const fn max_value() -> usize {
        align_bits::<T>()
    }
}

// see https://github.com/rust-lang/rust/pull/94640
/// Safety:
/// The caller must ensure that the start of `ptr`:
/// 1. does not point to memory that was previously allocated but is now deallocated;
/// 2. must be within the bounds of a single allocated object.
/// 3. ptr is not null?
unsafe fn ptr_len<T>(ptr: *const [T]) -> usize {
    (&*(ptr as *const [()])).len()
}

// implementation for slices
impl<T> PointerValuePair<[T]> {
    /// Creates a new `PointerValuePair` from the given raw pointer and extra bits.
    ///
    /// # Panics
    ///
    /// Panics if the pointer type `*const T` does not have enough available low bits to store
    /// the value.
    pub fn new_slice(ptr: *const [T], value: usize) -> PointerValuePair<[T]> {
        let m = align_bits::<T>();
        assert!(
            value <= m,
            "not enough alignment bits ({}) to store the value ({})",
            Self::available_bits(),
            value
        );

        let pv = unsafe {
            let len = ptr_len(ptr);
            let mut repr = ptr as *const T as usize;
            repr |= value;
            ptr::slice_from_raw_parts(repr as *const T, len)
        };

        PointerValuePair { pv }
    }

    /// Returns the pointer.
    pub fn ptr(self) -> *const [T] {
        unsafe {
            let len = ptr_len(self.pv);
            ptr::slice_from_raw_parts((self.pv as *const T as usize & !align_bits::<T>()) as *const T, len)
        }
    }

    /// Returns the value stored alongside the pointer.
    pub fn value(self) -> usize {
        self.pv as *const T as usize & align_bits::<T>()
    }

    /// Returns the number of bits available to store the value.
    pub const fn available_bits() -> u32 {
        align_bits::<T>().count_ones()
    }

    /// Returns the maximum (inclusive) integer value that can be stored in the pointer.
    pub const fn max_value() -> usize {
        align_bits::<T>()
    }
}

/// Trait that provides a generic way to access the value stored in a pointer-value pair, regardless of
/// whether it points to a single element (`&T where T: Sized`) or a slice (`&[T]`).
pub trait PointerValuePairAccess: Copy {
    type Target: ?Sized;

    /// Returns the stored pointer.
    fn ptr(self) -> *const Self::Target;
    /// Returns the stored pointer as a mutable raw pointer.
    fn mut_ptr(self) -> *mut Self::Target;
    /// Returns the value stored alongside the pointer.
    fn value(self) -> usize;
    /// Returns the number of bits available to store the value.
    fn available_bits() -> u32;
    /// Returns the maximum (inclusive) integer value that can be stored in the pointer.
    fn max_value() -> usize;
}

impl<T> PointerValuePairAccess for PointerValuePair<T> {
    type Target = T;

    fn ptr(self) -> *const T {
        self.ptr()
    }

    fn mut_ptr(self) -> *mut T {
        self.ptr() as *mut T
    }

    fn value(self) -> usize {
        self.value()
    }

    fn available_bits() -> u32 {
        Self::available_bits()
    }

    fn max_value() -> usize {
        Self::max_value()
    }
}

impl<T> PointerValuePairAccess for PointerValuePair<[T]> {
    type Target = [T];

    fn ptr(self) -> *const [T] {
        self.ptr()
    }

    fn mut_ptr(self) -> *mut [T] {
        self.ptr() as *mut [T]
    }

    fn value(self) -> usize {
        self.value()
    }

    fn available_bits() -> u32 {
        Self::available_bits()
    }

    fn max_value() -> usize {
        Self::max_value()
    }
}

#[cfg(test)]
mod tests {
    use super::PointerValuePair;
    use std::mem;

    #[test]
    fn pointer_sized() {
        assert_eq!(mem::size_of::<*const i32>(), mem::size_of::<PointerValuePair<i32>>());
    }

    #[test]
    fn basic_get_set() {
        let pointee = 42usize;
        let pv = PointerValuePair::new(&pointee, 3);
        assert_eq!(pv.ptr(), &pointee as *const _);
        let p_val = unsafe { *pv.ptr() };
        assert_eq!(p_val, 42usize);
        assert_eq!(pv.value(), 3);
    }

    #[test]
    fn custom_alignments() {
        #[repr(C, align(8))]
        struct Align8(i32);
        assert!(PointerValuePair::<Align8>::max_value() >= 0x7);
        assert!(PointerValuePair::<Align8>::available_bits() >= 3);

        #[repr(C, align(16))]
        struct Align16(i32);
        assert!(PointerValuePair::<Align16>::max_value() >= 0xF);
        assert!(PointerValuePair::<Align16>::available_bits() >= 4);

        #[repr(C, align(32))]
        struct Align32(i32);
        assert!(PointerValuePair::<Align32>::max_value() >= 0x1F);
        assert!(PointerValuePair::<Align32>::available_bits() >= 5);
    }

    #[test]
    fn slices() {
        let s = &[0, 1, 2, 3, 4, 5];
        let pv = PointerValuePair::new_slice(&s[..], 3);
        assert_eq!(pv.ptr(), &s[..]);
        assert_eq!(unsafe { &*pv.ptr() }, s);
        assert_eq!(pv.value(), 3);
    }
}
