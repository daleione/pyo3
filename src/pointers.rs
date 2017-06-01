// Copyright (c) 2017-present PyO3 Project and Contributors

use std;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::convert::{AsRef, AsMut};

use ffi;
use err::{PyResult, PyDowncastError};
use conversion::{ToPyObject, IntoPyObject};
use objects::{PyObject, PyObjectPtr};
use python::{Python, ToPythonPointer, IntoPythonPointer};
use token::PythonObjectWithGilToken;
use typeob::{PyTypeInfo, PyObjectAlloc};


#[allow(non_camel_case_types)]
pub struct PyPtr(*mut ffi::PyObject);

// `PyPtr` is thread-safe, because any python related operations require a Python<'p> token.
unsafe impl Send for PyPtr {}
unsafe impl Sync for PyPtr {}


impl PyPtr {
    /// Creates a `PyObjectPtr` instance for the given FFI pointer.
    /// This moves ownership over the pointer into the `PyObjectPtr`.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_owned_ptr(ptr: *mut ffi::PyObject) -> PyPtr {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        PyPtr(ptr)
    }

    /// Creates a `PyObjectPtr` instance for the given Python FFI pointer.
    /// Calls Py_INCREF() on the ptr.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_borrowed_ptr(ptr: *mut ffi::PyObject) -> PyPtr {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        ffi::Py_INCREF(ptr);
        PyPtr::from_owned_ptr(ptr)
    }

    /// Gets the reference count of the PyObject pointer.
    #[inline]
    pub fn get_refcnt(&self) -> isize {
        unsafe { ffi::Py_REFCNT(self.0) }
    }

    /// Get reference to &PyObject<'p>
    #[inline]
    pub fn as_object<'p>(&self, _py: Python<'p>) -> &PyObject<'p> {
        unsafe { std::mem::transmute(self) }
    }

    /// Converts `PyPtr` instance -> PyObject<'p>
    /// Consumes `self` without calling `Py_DECREF()`
    #[inline]
    pub fn into_object<'p>(self, _py: Python<'p>) -> PyObject<'p> {
        unsafe { std::mem::transmute(self) }
    }

    /// Converts `PyPtr` instance -> PyObjectPtr
    /// Consumes `self` without calling `Py_DECREF()`
    #[inline]
    pub fn into_object_ptr(self) -> PyObjectPtr {
        unsafe { std::mem::transmute(self) }
    }

    /// Clone self, Calls Py_INCREF() on the ptr.
    #[inline]
    pub fn clone_ref(&self, _py: Python) -> PyPtr {
        unsafe { PyPtr::from_borrowed_ptr(self.0) }
    }

    /// Casts the `PyObjectPtr` imstance to a concrete Python object type.
    /// Fails with `PyDowncastError` if the object is not of the expected type.
    #[inline]
    pub fn cast_into<'p, D>(self, py: Python<'p>) -> Result<D, PyDowncastError<'p>>
        where D: ::PyDowncastInto<'p>
    {
        <D as ::PyDowncastInto>::downcast_into(py, self)
    }
}

impl ToPythonPointer for PyPtr {
    /// Gets the underlying FFI pointer, returns a borrowed pointer.
    #[inline]
    fn as_ptr(&self) -> *mut ffi::PyObject {
        self.0
    }
}

impl IntoPythonPointer for PyPtr {
    /// Gets the underlying FFI pointer, returns a owned pointer.
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut ffi::PyObject {
        let ptr = self.0;
        std::mem::forget(self);
        ptr
    }
}

impl PartialEq for PyPtr {
    #[inline]
    fn eq(&self, o: &PyPtr) -> bool {
        self.0 == o.0
    }
}


/// Dropping a `PyPtr` instance decrements the reference count on the object by 1.
impl Drop for PyPtr {

    fn drop(&mut self) {
        unsafe {
            println!("drop PyObjectPtr: {:?} {} {:?}",
                     self.0, ffi::Py_REFCNT(self.0), &self as *const _);
        }
        let _gil_guard = Python::acquire_gil();
        unsafe { ffi::Py_DECREF(self.0); }
    }
}


pub struct Py<'p, T> {
    pub inner: *mut ffi::PyObject,
    _t: PhantomData<T>,
    py: Python<'p>,
}

impl<'p, T> Py<'p, T>
{
    /// Creates a Py instance for the given FFI pointer.
    /// This moves ownership over the pointer into the Py.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_owned_ptr(py: Python<'p>, ptr: *mut ffi::PyObject) -> Py<'p, T> {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        Py {inner: ptr, _t: PhantomData, py: py}
    }

    /// Creates a Py instance for the given FFI pointer.
    /// Calls Py_INCREF() on the ptr.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_borrowed_ptr(py: Python<'p>, ptr: *mut ffi::PyObject) -> Py<'p, T> {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        ffi::Py_INCREF(ptr);
        Py {inner: ptr, _t: PhantomData, py: py}
    }

    /// Returns owned PyObject<'p> reference
    #[inline]
    pub fn as_object(&self) -> &'p PyObject<'p> {
        unsafe { std::mem::transmute(self) }
    }

    /// Converts Py<'p, T> -> PyObject<'p>
    /// Consumes `self` without calling `Py_DECREF()`
    #[inline]
    pub fn into_object(self) -> PyObject<'p> {
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn gil(&self) -> Python<'p> {
        self.py
    }
}

impl<'p, T> Py<'p, T> where T: PyTypeInfo
{
    /// Create new python object and move T instance under python management
    pub fn new(py: Python<'p>, value: T) -> PyResult<Py<'p, T>> where T: PyObjectAlloc<Type=T>
    {
        let ob = unsafe {
            try!(<T as PyObjectAlloc>::alloc(py, value))
        };
        Ok(Py{inner: ob, _t: PhantomData, py: py})
    }

    #[inline]
    pub fn as_ref(&self) -> &T {
        let offset = <T as PyTypeInfo>::offset();
        unsafe {
            let ptr = (self.inner as *mut u8).offset(offset) as *mut T;
            ptr.as_ref().unwrap()
        }
    }

    #[inline]
    pub fn as_mut(&self) -> &mut T {
        let offset = <T as PyTypeInfo>::offset();
        unsafe {
            let ptr = (self.inner as *mut u8).offset(offset) as *mut T;
            ptr.as_mut().unwrap()
        }
    }
}

impl<'p, T> PythonObjectWithGilToken<'p> for Py<'p, T> {
    fn gil(&self) -> Python<'p> {
        self.py
    }
}

impl<'p, T> ToPythonPointer for Py<'p, T> {
    /// Gets the underlying FFI pointer, returns a borrowed pointer.
    #[inline]
    fn as_ptr(&self) -> *mut ffi::PyObject {
        self.inner
    }
}

impl<'p, T> IntoPythonPointer for Py<'p, T> {

    /// Gets the underlying FFI pointer.
    /// Consumes `self` without calling `Py_DECREF()`, thus returning an owned pointer.
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut ffi::PyObject {
        let ptr = self.inner;
        std::mem::forget(self);
        ptr
    }
}

/// Dropping a `Py` instance decrements the reference count on the object by 1.
impl<'p, T> Drop for Py<'p, T> {
    fn drop(&mut self) {
        unsafe {
            println!("drop Py: {:?} {} {:?}",
                     self.inner,
                     ffi::Py_REFCNT(self.inner), &self as *const _);
        }
        unsafe { ffi::Py_DECREF(self.inner); }
    }
}

impl<'p, T> Clone for Py<'p, T> {
    fn clone(&self) -> Self {
        unsafe {
            debug_assert!(!self.inner.is_null() && ffi::Py_REFCNT(self.inner) > 0);
            ffi::Py_INCREF(self.inner);
            Py {inner: self.inner, _t: PhantomData, py: self.py}
        }
    }
}

impl<'p, T> Deref for Py<'p, T> where T: PyTypeInfo {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<'p, T> DerefMut for Py<'p, T> where T: PyTypeInfo {

    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<'p, T> AsRef<T> for Py<'p, T> where T: PyTypeInfo {
    #[inline]
    fn as_ref(&self) -> &T {
        self.as_ref()
    }
}

impl<'p, T> AsMut<T> for Py<'p, T> where T: PyTypeInfo {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        Py::<T>::as_mut(self)
    }
}

impl <'a, T> ToPyObject for Py<'a, T> {

    #[inline]
    fn to_object<'p>(&self, py: Python<'p>) -> PyObject<'p> {
        PyObject::from_borrowed_ptr(py, self.inner)
    }

    #[inline]
    fn with_borrowed_ptr<F, R>(&self, _py: Python, f: F) -> R
        where F: FnOnce(*mut ffi::PyObject) -> R
    {
        f(self.inner)
    }
}

impl<'p, T> IntoPyObject for Py<'p, T> {

    #[inline]
    default fn into_object(self, _py: Python) -> ::PyObjectPtr {
        unsafe { std::mem::transmute(self) }
    }
}

/// PyObject implements the `==` operator using reference equality:
/// `obj1 == obj2` in rust is equivalent to `obj1 is obj2` in Python.
impl<'p, T> PartialEq for Py<'p, T> {
    #[inline]
    fn eq(&self, o: &Py<T>) -> bool {
        self.as_ptr() == o.as_ptr()
    }
}