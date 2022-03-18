use std::{marker::PhantomData, mem};

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
pub struct PointerValuePair<T> {
    repr: usize,
    _phantom: PhantomData<T>,
}

impl<T> Copy for PointerValuePair<T> {}

impl<T> Clone for PointerValuePair<T> {
    fn clone(&self) -> Self {
        PointerValuePair {
            repr: self.repr,
            _phantom: PhantomData,
        }
    }
}

const fn align_bits_mask<T>() -> usize {
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

        let m = align_bits_mask::<T>();
        assert!(
            value <= m,
            "not enough alignment bits ({}) to store the value ({})",
            Self::available_bits(),
            value
        );

        let mut repr = ptr as usize;
        repr |= value;

        PointerValuePair {
            repr,
            _phantom: PhantomData,
        }
    }

    /// Returns the number of bits available to store the value.
    pub const fn available_bits() -> u32 {
        align_bits_mask::<T>().count_ones()
    }

    /// Returns the maximum (inclusive) integer value that can be stored in the pointer.
    pub const fn max_value() -> usize {
        align_bits_mask::<T>()
    }

    /// Returns the pointer.
    pub const fn ptr(self) -> *const T {
        (self.repr & !align_bits_mask::<T>()) as *const T
    }

    /// Returns the value stored alongside the pointer.
    pub const fn value(self) -> usize {
        self.repr & align_bits_mask::<T>()
    }
}


#[cfg(test)]
mod tests {
    use super::PointerValuePair;

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
}
