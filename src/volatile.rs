use core::{
    any::type_name,
    borrow::{Borrow, BorrowMut},
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    slice,
};

use crate::{VolatileData, VolatileRead, VolatileWrite};

#[derive(Debug)]
pub struct ReadWrite;
#[derive(Debug)]
pub struct ReadOnly;
#[derive(Debug)]
pub struct WriteOnly;

pub trait Read {}
impl Read for ReadWrite {}
impl Read for ReadOnly {}

pub trait Write {}
impl Write for ReadWrite {}
impl Write for WriteOnly {}

/// Volatile data or memory.
///
/// See [crate-level documentation](crate) for details.
#[repr(C)]
pub union Volatile<T: Copy, Permission = ReadWrite> {
    _data: T,
    _perm: PhantomData<Permission>,
}

/// Volatile read-only data or memory.
///
/// See [crate-level documentation](crate) for details.
///
/// See [`Volatile<T>`] for methods and [methods](Volatile<T>#implementations)
/// and [trait implementations](Volatile<T>#trait-implementations).
pub type VolatileReadOnly<T> = Volatile<T, ReadOnly>;

/// Volatile write-only data or memory.
///
/// See [crate-level documentation](crate) for details.
///
/// See [`Volatile<T>`] for methods and [methods](Volatile<T>#implementations)
/// and [trait implementations](Volatile<T>#trait-implementations).
pub type VolatileWriteOnly<T> = Volatile<T, WriteOnly>;

impl<T: Copy, P> Volatile<T, P> {
    /// Converts a pointer to `T` into a reference to `Volatile<T>`, which can
    /// be [read-only](VolatileReadOnly), [write-only](VolatileWriteOnly), or
    /// both readable and writable (the default).
    ///
    /// # Safety
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `mem` must be [valid](core::ptr#safety) for reads and/or writes.
    ///
    /// - `mem` must be properly aligned.
    ///
    /// - `mem` must point to a properly initialized value of type `T` (unless
    /// the resulting `Volatile<T>` is [write-only](VolatileWriteOnly)).
    ///
    /// Note that even if `T` has size zero, the pointer must be non-NULL and
    /// properly aligned.
    ///
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a write operation any other
    /// operation (reading or writing) to the same location is undefined
    /// behavior.
    pub unsafe fn from_ptr<'a>(mem: *const T) -> &'a Self {
        // SAFETY: The caller must ensure the pointer is safe to use. It is
        // safe to cast to `*const Self` because `Self` is transparent.
        unsafe { &*(mem as *const Self) }
    }

    /// Converts a mutable pointer to `T` into a mutable reference to
    /// `Volatile<T>`, which can be [read-only](VolatileReadOnly),
    /// [write-only](VolatileWriteOnly), or both readable and writable (the
    /// default).
    ///
    /// # Safety
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `mem` must be [valid](core::ptr#safety) for reads and/or writes.
    ///
    /// - `mem` must be properly aligned.
    ///
    /// - `mem` must point to a properly initialized value of type `T` (unless
    /// the resulting `Volatile<T>` is [write-only](VolatileWriteOnly)).
    ///
    /// Note that even if `T` has size zero, the pointer must be non-NULL and
    /// properly aligned.
    pub unsafe fn from_mut_ptr<'a>(mem: *mut T) -> &'a mut Self {
        // SAFETY: The caller must ensure the pointer is safe to use. It is
        // safe to cast to `*mut Self` because `Self` is transparent.
        unsafe { &mut *(mem as *mut Self) }
    }

    /// Converts a shared reference to `T` into a shared reference to
    /// `Volatile<T>`, which can be [read-only](VolatileReadOnly),
    /// [write-only](VolatileWriteOnly), or both readable and writable (the
    /// default).
    pub fn from_ref<'a>(mem: &T) -> &'a Self {
        // SAFETY: `mem` is a reference to a `Copy` type. It is safe to cast to
        // `*const Self` because `Self` is transparent.
        unsafe { &*(mem as *const T as *const Volatile<T, P>) }
    }

    /// Converts a mutable reference to `T` into a mutable reference to
    /// `Volatile<T>`, which can be [read-only](VolatileReadOnly),
    /// [write-only](VolatileWriteOnly), or both readable and writable (the
    /// default).
    pub fn from_mut<'a>(mem: &mut T) -> &'a mut Self {
        // SAFETY: `mem` is a mutable reference to a `Copy` type. It is safe to
        // cast to `*mut Self` because `Self` is transparent.
        unsafe { &mut *(mem as *mut T as *mut Volatile<T, P>) }
    }
}

impl<'a, T: Copy, P> From<&'a T> for &'a Volatile<T, P> {
    fn from(mem: &'a T) -> &'a Volatile<T, P> {
        Volatile::from_ref(mem)
    }
}

impl<'a, T: Copy, P> From<&'a mut T> for &'a mut Volatile<T, P> {
    fn from(mem: &'a mut T) -> &'a mut Volatile<T, P> {
        Volatile::from_mut(mem)
    }
}

impl<T: Copy, P> VolatileData<T> for Volatile<T, P> {}

impl<T: Copy, P: Read> VolatileRead<T> for Volatile<T, P> {
    /// Performs a volatile read of the value in `self` without moving it. This
    /// leaves the memory in `self` unchanged.
    fn read(&self) -> T {
        // SAFETY: `self` is a reference. It is safe to cast to `*const T`
        // because `Self` is transparent. `T` is safe to read since it is `Copy`
        // and guaranteed to be initialized.
        unsafe { (self as *const _ as *const T).read_volatile() }
    }
}

impl<T: Copy, P: Write> VolatileWrite<T> for Volatile<T, P> {
    /// Performs a volatile write of `self` with the given value without reading
    /// the old value.
    fn write(&mut self, val: T) {
        // SAFETY: `self` is a mutable reference. It is safe to cast to `*mut T`
        // because `Self` is transparent. `T` is safe to write since it is
        // `Copy`.
        unsafe { (self as *mut _ as *mut T).write_volatile(val) }
    }
}

impl<T: Copy, P, const N: usize> Deref for Volatile<[T; N], P> {
    type Target = [Volatile<T, P>];

    fn deref(&self) -> &Self::Target {
        let ptr = self as *const _ as *const Volatile<T, P>;
        // SAFETY: `ptr` is valid for N elements of `Volatile<T>`, because it
        // comes from a reference to `Volatile<[T; N]>` and `Volatile` is
        // transparent.
        unsafe { slice::from_raw_parts(ptr, N) }
    }
}

impl<T: Copy, P, const N: usize> DerefMut for Volatile<[T; N], P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = self as *mut _ as *mut Volatile<T, P>;
        // SAFETY: `ptr` is valid for N elements of `Volatile<T>`, because it
        // comes from a reference to `Volatile<[T; N]>` and `Volatile` is
        // transparent.
        unsafe { slice::from_raw_parts_mut(ptr, N) }
    }
}

impl<T: Copy, P, const N: usize> Borrow<[Volatile<T, P>]> for Volatile<[T; N], P> {
    fn borrow(&self) -> &[Volatile<T, P>] {
        self
    }
}

impl<T: Copy, P, const N: usize> BorrowMut<[Volatile<T, P>]> for Volatile<[T; N], P> {
    fn borrow_mut(&mut self) -> &mut [Volatile<T, P>] {
        self
    }
}

impl<T: Copy, P, const N: usize> AsRef<[Volatile<T, P>]> for Volatile<[T; N], P> {
    fn as_ref(&self) -> &[Volatile<T, P>] {
        self
    }
}

impl<T: Copy, P, const N: usize> AsMut<[Volatile<T, P>]> for Volatile<[T; N], P> {
    fn as_mut(&mut self) -> &mut [Volatile<T, P>] {
        self
    }
}

impl<T: Copy, P> fmt::Debug for Volatile<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(type_name::<Self>())
    }
}
