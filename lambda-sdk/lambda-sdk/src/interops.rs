use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
    str::from_utf8_unchecked,
};

#[repr(transparent)]
pub struct Pointer<'r, T: ?Sized, const MUTABILITY: bool = false> {
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'r mut T>,
}

impl<'r, T: ?Sized, const MUTABILITY: bool> Pointer<'r, T, MUTABILITY> {
    pub fn shrink_lifetime<'t>(self) -> Pointer<'t, T, MUTABILITY> {
        Pointer {
            ptr: self.ptr,
            _lifetime: PhantomData,
        }
    }

    pub fn borrow_as_ref(&'_ self) -> Pointer<'_, T, false> {
        Pointer {
            ptr: self.ptr,
            _lifetime: PhantomData,
        }
    }

    pub fn into_ref<'t>(self) -> &'t T
    where
        'r: 't,
    {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_ref() }
    }
}

impl<'r, T: ?Sized> Pointer<'r, T, true> {
    pub fn borrow_as_mut(&'_ mut self) -> Pointer<'_, T, true> {
        Pointer {
            ptr: self.ptr,
            _lifetime: PhantomData,
        }
    }

    pub fn into_mut<'t>(mut self) -> &'t mut T
    where
        'r: 't,
    {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_mut() }
    }
}

impl<'r, 't, T: ?Sized> From<&'r T> for Pointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: &'r T) -> Self {
        Self {
            ptr: NonNull::from(value),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T: ?Sized, const MUTABILITY: bool> From<&'r mut T> for Pointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: &'r mut T) -> Self {
        Self {
            ptr: NonNull::from(value),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T: ?Sized> From<Pointer<'r, T, true>> for Pointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: Pointer<'r, T, true>) -> Self {
        Self {
            ptr: value.ptr,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<SlicePointer<'r, T, MUTABILITY>>
    for Pointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: SlicePointer<'r, T, MUTABILITY>) -> Self {
        Self {
            ptr: value.ptr,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T> From<SlicePointer<'r, T, true>> for Pointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: SlicePointer<'r, T, true>) -> Self {
        Self {
            ptr: value.ptr,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<SlicePointer<'r, T, MUTABILITY>>
    for Pointer<'t, [T], MUTABILITY>
where
    'r: 't,
{
    fn from(value: SlicePointer<'r, T, MUTABILITY>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe {
            Self {
                ptr: NonNull::from(from_raw_parts(value.ptr.as_ptr(), value.length)),
                _lifetime: PhantomData,
            }
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<Pointer<'r, [T], MUTABILITY>> for Pointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: Pointer<'r, [T], MUTABILITY>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe {
            Self {
                ptr: NonNull::from(&value.ptr.as_ref()[0]),
                _lifetime: PhantomData,
            }
        }
    }
}

impl<'r, 't, T> From<Pointer<'r, [T], true>> for Pointer<'t, T, true>
where
    'r: 't,
{
    fn from(mut value: Pointer<'r, [T], true>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe {
            Self {
                ptr: NonNull::from(&mut value.ptr.as_mut()[0]),
                _lifetime: PhantomData,
            }
        }
    }
}

impl<'r, 't, T> From<SlicePointer<'r, T, true>> for Pointer<'t, [T], false>
where
    'r: 't,
{
    fn from(value: SlicePointer<'r, T, true>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe {
            Self {
                ptr: NonNull::from(from_raw_parts(value.ptr.as_ptr(), value.length)),
                _lifetime: PhantomData,
            }
        }
    }
}

impl<'r, 't> From<StringPointer<'r>> for Pointer<'t, u8, false>
where
    'r: 't,
{
    fn from(value: StringPointer<'r>) -> Self {
        From::from(value.ptr)
    }
}

impl<'r, 't> From<StringPointer<'r>> for Pointer<'t, [u8], false>
where
    'r: 't,
{
    fn from(value: StringPointer<'r>) -> Self {
        From::from(value.ptr)
    }
}

impl<'r, T> Clone for Pointer<'r, T, false> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'r, T> Copy for Pointer<'r, T, false> {}

impl<'r, T: ?Sized, const MUTABILITY: bool> Deref for Pointer<'r, T, MUTABILITY> {
    type Target = T;

    fn deref(&self) -> &T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_ref() }
    }
}

impl<'r, T: ?Sized> DerefMut for Pointer<'r, T, true> {
    fn deref_mut(&mut self) -> &mut T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_mut() }
    }
}

impl<'r, T: ?Sized, const MUTABILITY: bool> AsRef<T> for Pointer<'r, T, MUTABILITY> {
    fn as_ref(&self) -> &T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_ref() }
    }
}

impl<'r, T: ?Sized> AsMut<T> for Pointer<'r, T, true> {
    fn as_mut(&mut self) -> &mut T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_mut() }
    }
}

impl<'r, T: ?Sized, const MUTABILITY: bool> Borrow<T> for Pointer<'r, T, MUTABILITY> {
    fn borrow(&self) -> &T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_ref() }
    }
}

impl<'r, T: ?Sized> BorrowMut<T> for Pointer<'r, T, true> {
    fn borrow_mut(&mut self) -> &mut T {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { self.ptr.as_mut() }
    }
}

#[repr(packed, C)]
pub struct SlicePointer<'r, T, const MUTABILITY: bool = false> {
    ptr: NonNull<T>,
    length: usize,
    _lifetime: PhantomData<&'r mut [T]>,
}

impl<'r, T, const MUTABILITY: bool> SlicePointer<'r, T, MUTABILITY> {
    pub fn truncate(mut self, length: usize) -> Self {
        self.length = self.length.min(length);

        self
    }

    pub fn shrink_lifetime<'t>(self) -> SlicePointer<'t, T, MUTABILITY> {
        SlicePointer {
            ptr: self.ptr,
            length: self.length,
            _lifetime: PhantomData,
        }
    }

    pub fn borrow_as_ref(&'_ self) -> SlicePointer<'_, T, false> {
        SlicePointer {
            ptr: self.ptr,
            length: self.length,
            _lifetime: PhantomData,
        }
    }

    pub fn into_ref<'t>(self) -> &'t [T]
    where
        'r: 't,
    {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T> SlicePointer<'r, T, true> {
    pub fn borrow_as_mut(&'_ mut self) -> SlicePointer<'_, T, true> {
        SlicePointer {
            ptr: self.ptr,
            length: self.length,
            _lifetime: PhantomData,
        }
    }

    pub fn into_mut<'t>(self) -> &'t mut [T]
    where
        'r: 't,
    {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, 't, T> From<&'r [T]> for SlicePointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: &'r [T]) -> Self {
        Self {
            ptr: NonNull::from(value).cast(),
            length: value.len(),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<&'r mut [T]> for SlicePointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: &'r mut [T]) -> Self {
        let length: usize = value.len();

        Self {
            ptr: NonNull::from(value).cast(),
            length,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const N: usize> From<&'r [T; N]> for SlicePointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: &'r [T; N]) -> Self {
        Self {
            ptr: NonNull::from(value).cast(),
            length: value.len(),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const N: usize, const MUTABILITY: bool> From<&'r mut [T; N]>
    for SlicePointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: &'r mut [T; N]) -> Self {
        let length: usize = value.len();

        Self {
            ptr: NonNull::from(value).cast(),
            length,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T> From<SlicePointer<'r, T, true>> for SlicePointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: SlicePointer<'r, T, true>) -> Self {
        Self {
            ptr: value.ptr,
            length: value.length,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<Pointer<'r, T, MUTABILITY>>
    for SlicePointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: Pointer<'r, T, MUTABILITY>) -> Self {
        Self {
            ptr: value.ptr,
            length: 1,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T> From<Pointer<'r, T, true>> for SlicePointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: Pointer<'r, T, true>) -> Self {
        Self {
            ptr: value.ptr,
            length: 1,
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T, const MUTABILITY: bool> From<Pointer<'r, [T], MUTABILITY>>
    for SlicePointer<'t, T, MUTABILITY>
where
    'r: 't,
{
    fn from(value: Pointer<'r, [T], MUTABILITY>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        let slice: &[T] = unsafe { value.ptr.as_ref() };

        Self {
            ptr: NonNull::from(slice).cast(),
            length: slice.len(),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't, T> From<Pointer<'r, [T], true>> for SlicePointer<'t, T, false>
where
    'r: 't,
{
    fn from(value: Pointer<'r, [T], true>) -> Self {
        // # Safety
        // Safe because can only be created through a reference.
        let slice: &[T] = unsafe { value.ptr.as_ref() };

        Self {
            ptr: NonNull::from(slice).cast(),
            length: slice.len(),
            _lifetime: PhantomData,
        }
    }
}

impl<'r, 't> From<StringPointer<'r>> for SlicePointer<'t, u8, false>
where
    'r: 't,
{
    fn from(value: StringPointer<'r>) -> Self {
        value.ptr
    }
}

impl<'r, T> Clone for SlicePointer<'r, T, false> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'r, T> Copy for SlicePointer<'r, T, false> {}

impl<'r, T, const MUTABILITY: bool> Deref for SlicePointer<'r, T, MUTABILITY> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T> DerefMut for SlicePointer<'r, T, true> {
    fn deref_mut(&mut self) -> &mut [T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T, const MUTABILITY: bool> AsRef<[T]> for SlicePointer<'r, T, MUTABILITY> {
    fn as_ref(&self) -> &[T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T> AsMut<[T]> for SlicePointer<'r, T, true> {
    fn as_mut(&mut self) -> &mut [T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T, const MUTABILITY: bool> Borrow<[T]> for SlicePointer<'r, T, MUTABILITY> {
    fn borrow(&self) -> &[T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
}

impl<'r, T> BorrowMut<[T]> for SlicePointer<'r, T, true> {
    fn borrow_mut(&mut self) -> &mut [T] {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.length) }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct StringPointer<'r> {
    ptr: SlicePointer<'r, u8, false>,
}

impl<'r> StringPointer<'r> {
    pub fn into_ref<'t>(self) -> &'t str
    where
        'r: 't,
    {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_utf8_unchecked(self.ptr.into_ref()) }
    }
}

impl<'r> From<&'r str> for StringPointer<'r> {
    fn from(value: &'r str) -> Self {
        Self {
            ptr: SlicePointer::from(value.as_bytes()),
        }
    }
}

impl<'r> From<&'r mut str> for StringPointer<'r> {
    fn from(value: &'r mut str) -> Self {
        Self {
            ptr: SlicePointer::from(value.as_bytes()),
        }
    }
}

impl<'r> Deref for StringPointer<'r> {
    type Target = str;

    fn deref(&self) -> &str {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_utf8_unchecked(&self.ptr) }
    }
}

impl<'r> AsRef<str> for StringPointer<'r> {
    fn as_ref(&self) -> &str {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_utf8_unchecked(&self.ptr) }
    }
}

impl<'r> Borrow<str> for StringPointer<'r> {
    fn borrow(&self) -> &str {
        // # Safety
        // Safe because can only be created through a reference.
        unsafe { from_utf8_unchecked(&self.ptr) }
    }
}

macro_rules! define_read {
    ($fn_ident: ident $(, id: $id: ident)?) => {
        pub(crate) fn $fn_ident<'r>(
            read_fn: for<'t> unsafe extern "C" fn(
                $($id: NonZeroU64,)?
                buf: Pointer<'t, u8, true>,
                buf_len: usize,
            ) -> usize,
            length_fn: unsafe extern "C" fn($($id: NonZeroU64)?) -> u64,
            $($id: NonZeroU64,)?
            length: &mut u64,
            buf: &'r mut [u8],
        ) -> &'r [u8] {
            let mut buf: SlicePointer<'_, u8, true> = SlicePointer::from(buf);

            let read_len: usize = unsafe {
                let buf_len: usize = buf.len();

                read_fn($($id, )?buf.borrow_as_mut().into(), buf_len)
            };

            if let Ok(read_len) = u64::try_from(read_len) {
                *length -= read_len;
            } else {
                *length = unsafe { length_fn($($id)?) };
            }

            buf.truncate(read_len).into_ref()
        }
    };
}

define_read!(read_with_id, id: id);
