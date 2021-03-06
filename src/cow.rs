use crate::{PointerValuePair, PointerValuePairAccess};
use std::{marker::PhantomData, mem, ops::Deref};

/// A pointer-sized object that holds either a borrow (`&'a T`) or a boxed value (`Box<T>`).
///
/// TODO doc: implements deref, construction, ToOwned, etc.
///
/// # Notes
///
/// Because it uses `PointerValuePair` internally, `T` cannot not be a zero-sized type.
#[repr(transparent)]
pub struct Cow<'a, T>
where
    T: ?Sized,
    PointerValuePair<T>: PointerValuePairAccess,
{
    inner: PointerValuePair<T>,
    _phantom: PhantomData<&'a T>,
}

const BORROWED: usize = 0usize;
const OWNED: usize = 1usize;

impl<'a, T> Cow<'a, T> {
    /// Creates a new `Cow` representing a borrowed value.
    pub fn borrowed(v: &'a T) -> Cow<'a, T> {
        Cow {
            inner: PointerValuePair::new(v, BORROWED),
            _phantom: PhantomData,
        }
    }

    /// Creates a new `Cow` holding a boxed value.
    pub fn owned(v: Box<T>) -> Cow<'a, T> {
        Cow {
            inner: PointerValuePair::new(Box::into_raw(v), OWNED),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> Cow<'a, T>
where
    T: Clone,
{
    /// Converts this `Cow` into a `Box<T>`. If this `Cow` is a borrow, clones the value and boxes it.
    pub fn into_owned(self) -> Box<T> {
        if self.inner.value() == OWNED {
            let boxed = unsafe {
                // SAFETY: the pointer has been created with `Box::into_raw` by `Cow::owned`.
                // We inhibit drop by calling mem::forget below.
                Box::from_raw(self.inner.ptr() as *mut T)
            };
            // we extracted the boxed value already, don't double-drop
            mem::forget(self);
            boxed
        } else {
            Box::new(self.deref().clone())
        }
    }

    /// Converts this `Cow` into an owned `Cow` by cloning the value and boxing it, if it is borrowed.
    pub fn into_owned_cow<'b>(self) -> Cow<'b, T> {
        if self.inner.value() == OWNED {
            // We own the value, so it's OK to just transfer it
            let result = Cow {
                inner: self.inner,
                _phantom: Default::default(),
            };
            // we transferred ownership of the box, don't double-drop
            mem::forget(self);
            result
        } else {
            Cow::owned(Box::new(self.deref().clone()))
        }
    }
}

impl<'a, T> Cow<'a, [T]> {
    /// Creates a new `Cow` representing a borrowed value.
    pub fn borrowed_slice(v: &'a [T]) -> Cow<'a, [T]> {
        Cow {
            inner: PointerValuePair::new_slice(v, BORROWED),
            _phantom: PhantomData,
        }
    }

    /// Creates a new `Cow` holding a boxed value.
    pub fn owned_slice(v: Box<[T]>) -> Cow<'a, [T]> {
        Cow {
            inner: PointerValuePair::new_slice(Box::into_raw(v), OWNED),
            _phantom: PhantomData,
        }
    }
}

// impl Cow<[T]>
impl<'a, T> Cow<'a, [T]>
where
    T: Copy,
{
    /// Converts this `Cow` into a boxed slice. If this `Cow` is a borrow, clones the slice and boxes it.
    pub fn into_owned_slice(self) -> Box<[T]> {
        if self.inner.value() == OWNED {
            let boxed = unsafe {
                // SAFETY: the pointer has been created with `Box::into_raw` by `Cow::owned`.
                // We inhibit drop by calling mem::forget below.
                Box::from_raw(self.inner.ptr() as *mut [T])
            };
            // we extracted the boxed value already, don't double-drop
            mem::forget(self);
            boxed
        } else {
            self.deref().into()
        }
    }

    /// Converts this `Cow` into an owned `Cow` by cloning the value and boxing it, if it is borrowed.
    pub fn into_owned_cow_slice<'b>(self) -> Cow<'b, [T]> {
        if self.inner.value() == OWNED {
            // We own the value, so it's OK to just transfer it
            let result = Cow {
                inner: self.inner,
                _phantom: Default::default(),
            };
            // we transferred ownership of the box, don't double-drop
            mem::forget(self);
            result
        } else {
            Cow::owned_slice(self.deref().into())
        }
    }
}

impl<'a, T> Drop for Cow<'a, T>
where
    T: ?Sized,
    PointerValuePair<T>: PointerValuePairAccess,
{
    fn drop(&mut self) {
        unsafe {
            if self.inner.value() == OWNED {
                drop(Box::from_raw(self.inner.mut_ptr()))
            }
        }
    }
}

impl<'a, T> Deref for Cow<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: ptr is either a pointer to a boxed value for which we are the owner (and are responsible for the deletion),
        // or a pointer to a borrowed value, whose validity is ensured by the lifetime bound.
        unsafe { &*self.inner.ptr() }
    }
}

impl<'a, T> Deref for Cow<'a, [T]> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SAFETY: ptr is either a pointer to a boxed value for which we are the owner (and are responsible for the deletion),
        // or a pointer to a borrowed value, whose validity is ensured by the lifetime bound.
        unsafe { &*self.inner.ptr() }
    }
}

impl<'a, T> From<&'a [T]> for Cow<'a, [T]> {
    /// Creates a borrowed `Cow<[T]>` from the given slice.
    fn from(slice: &'a [T]) -> Self {
        Cow::borrowed_slice(slice)
    }
}

#[cfg(test)]
mod tests {
    use crate::Cow;
    use std::{cell::Cell, mem};

    #[test]
    fn pointer_sized() {
        assert_eq!(mem::size_of::<*const i32>(), mem::size_of::<Cow<'static, i32>>());
    }

    #[test]
    fn owned_cow_drop() {
        let drop_flag = Cell::new(false);

        #[derive(Clone)]
        struct DropTest<'a> {
            flag: &'a Cell<bool>,
        }

        impl<'a> Drop for DropTest<'a> {
            fn drop(&mut self) {
                self.flag.set(true)
            }
        }

        let drop_test = DropTest { flag: &drop_flag };
        let cow = Cow::owned(Box::new(drop_test));
        let cow = cow.into_owned_cow();
        assert!(!drop_flag.get());
        let boxed = cow.into_owned();
        assert!(!drop_flag.get());
        let cow = Cow::owned(boxed);
        assert!(!drop_flag.get());
        drop(cow);
        assert!(drop_flag.get());

        //----------------------------------------------------------------
        drop_flag.set(false);
        let drop_test = DropTest { flag: &drop_flag };
        let cow = Cow::borrowed(&drop_test);
        drop(cow);
        assert!(!drop_flag.get());
        drop(drop_test);
        assert!(drop_flag.get());

        //----------------------------------------------------------------
        let drop_test = DropTest { flag: &drop_flag };
        let cow = Cow::borrowed(&drop_test);
        let cow = cow.into_owned_cow();
        drop(cow);
        assert!(drop_flag.get());
        drop_flag.set(false);
        drop(drop_test);
        assert!(drop_flag.get());
    }

    #[test]
    fn dst_cow_drop() {
        let drop_count = Cell::new(0usize);

        #[derive(Clone)]
        struct DropTest<'a> {
            count: &'a Cell<usize>,
        }

        impl<'a> Drop for DropTest<'a> {
            fn drop(&mut self) {
                let c = self.count.get();
                self.count.set(c + 1);
            }
        }

        let dt = DropTest { count: &drop_count };

        //----------------------------------------------------------------
        let cow = Cow::owned_slice([dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone()].into());
        assert_eq!(drop_count.get(), 0);
        drop(cow);
        assert_eq!(drop_count.get(), 6);

        //----------------------------------------------------------------
        drop_count.set(0);
        let slice = [dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone()];
        let borrowed_cow = Cow::borrowed_slice(&slice);
        drop(borrowed_cow);
        assert_eq!(drop_count.get(), 0);
        drop(slice);
        assert_eq!(drop_count.get(), 6);

        /*//----------------------------------------------------------------
        drop_count.set(0);
        let borrowed_cow =
            Cow::borrowed_slice(&[dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone(), dt.clone()]);
        let owned_cow = borrowed_cow.into_owned_cow_slice();
        drop(borrowed_cow);
        assert_eq!(drop_count.get(), 0);
        drop(owned_cow);
        assert_eq!(drop_count.get(), 6);*/
    }
}
