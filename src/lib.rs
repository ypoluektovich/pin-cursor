//! A simple `!Unpin` I/O backend for async-std, designed for use in tests.
//!
//! This crate provides the `PinCursor` struct which wraps around `async_std::io::Cursor`
//! but is explicitly **not** `Unpin`. It is a little building block to help write tests
//! where you want to ensure that your own asynchronous IO code behaves correctly when reading from
//! or writing to something that is *definitely* `!Unpin`.
//!
//! - It can be backed by any `Unpin` data buffer that can be slotted into `async_std::io::Cursor`.
//!   Usually `Vec<u8>` or `&mut [u8]` (e. g. from an array) are used.
//! - It implements `async_std::io::{Read, Write, Seek}`, so you can poll these traits' methods
//!   in your own futures.
//! - At the same time, it provides several high-level methods through which you can manipulate
//!   the PinCursor in a simple `async {}` block.
//!
//! # Examples
//!
//! ```
//! # use async_std::task::block_on;
//! use pin_cursor::PinCursor;
//! use async_std::io::{prelude::*, Cursor};
//! use std::io::SeekFrom;
//! use std::pin::Pin;
//!
//! // Construct a async_std::io::Cursor however you like...
//! let mut data: Vec<u8> = Vec::new();
//! let cursor = Cursor::new(&mut data);
//! // ... then wrap it in PinCursor and a pinned pointer, thus losing the Unpin privileges.
//! let mut cursor: Pin<Box<PinCursor<_>>> = Box::pin(PinCursor::wrap(cursor));
//! // Note that we have to make an owning pointer first -
//! // making a Pin<&mut PinCursor<_>> directly is impossible!
//! // (There is a more complex way to allocate on stack - see the features section.)
//!
//! // Methods of PinCursor mostly return futures and are designed for use in async contexts.
//! # block_on(
//! async {
//!     // You can write!
//!     assert_eq!(cursor.as_mut().write(&[1u8, 2u8, 3u8]).await.unwrap(), 3);
//! 
//!     // You can seek!
//!     assert_eq!(cursor.position(), 3);
//!     assert_eq!(cursor.as_mut().seek(SeekFrom::Start(1)).await.unwrap(), 1);
//!     assert_eq!(cursor.position(), 1);
//! 
//!     // You can read!
//!     let mut buf = [0u8; 1];
//!     assert_eq!(cursor.as_mut().read(buf.as_mut()).await.unwrap(), 1);
//!     assert_eq!(buf[0], 2);
//!
//!     // There's also this way of seeking that doesn't involve futures.
//!     cursor.as_mut().set_position(0);
//!     assert_eq!(cursor.as_mut().read(buf.as_mut()).await.unwrap(), 1);
//!     assert_eq!(buf[0], 1);
//! }
//! # );
//! ```
//!
//! # Features
//!
//! The optional feature `stackpin` enables integration with [stackpin], a crate that provides
//! a way to allocate `!Unpin` structures on stack.
//!
//! ```ignore
//! # use pin_cursor::PinCursor;
//! # use async_std::io::Cursor;
//! # use std::pin::Pin;
//! use stackpin::stack_let;
//!
//! let mut data: Vec<u8> = vec![1, 2];
//! stack_let!(mut cursor : PinCursor<_> = Cursor::new(&mut data));
//! let cursor_ptr: Pin<&mut PinCursor<_>> = Pin::as_mut(&mut cursor);
//! ```
//!
//! Now you have a correctly pinned `PinCursor` that's allocated on stack instead of in a box.
//!
//! [stackpin]: https://docs.rs/stackpin/0.0.2

use std::future::Future;
use std::io::{IoSlice, IoSliceMut, Result, SeekFrom};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_std::io::Cursor;
use async_std::io::prelude::*;
use pin_project_lite::pin_project;

#[cfg(feature = "stackpin")]
mod impl_stackpin;

pin_project! {
    pub struct PinCursor<T> {
        c: Cursor<T>,
        #[pin]
        _p: PhantomPinned
    }
}

impl<T> PinCursor<T>
    where T: Unpin,
          Cursor<T>: Write + Read + Seek
{
    pub fn wrap(c: Cursor<T>) -> Self {
        Self { c, _p: PhantomPinned }
    }

    pub fn unwrap(self) -> Cursor<T> {
        self.c
    }

    pub fn position(&self) -> u64 {
        self.c.position()
    }

    pub fn set_position(self: Pin<&mut Self>, pos: u64) {
        self.project().c.set_position(pos)
    }

    pub fn write<'a>(self: Pin<&'a mut Self>, buf: &'a [u8]) -> impl Future<Output=Result<usize>> + 'a {
        self.project().c.write(buf)
    }

    pub fn read<'a>(self: Pin<&'a mut Self>, buf: &'a mut [u8]) -> impl Future<Output=Result<usize>> + 'a {
        self.project().c.read(buf)
    }

    pub fn seek(self: Pin<&mut Self>, pos: SeekFrom) -> impl Future<Output=Result<u64>> + '_ {
        self.project().c.seek(pos)
    }
}

impl<T> Read for PinCursor<T>
    where T: Unpin,
          Cursor<T>: Read
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        Pin::new(self.project().c).poll_read(cx, buf)
    }

    fn poll_read_vectored(self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &mut [IoSliceMut<'_>]) -> Poll<Result<usize>> {
        Pin::new(self.project().c).poll_read_vectored(cx, bufs)
    }
}

impl<T> Write for PinCursor<T>
    where T: Unpin,
          Cursor<T>: Write
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(self.project().c).poll_write(cx, buf)
    }

    fn poll_write_vectored(self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[IoSlice<'_>]) -> Poll<Result<usize>> {
        Pin::new(self.project().c).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(self.project().c).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(self.project().c).poll_close(cx)
    }
}

impl<T> Seek for PinCursor<T>
    where T: Unpin,
          Cursor<T>: Seek
{
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        Pin::new(self.project().c).poll_seek(cx, pos)
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::{assert_impl_all, assert_not_impl_all};

    use super::*;

    #[test]
    fn impls() {
        assert_not_impl_all!(PinCursor<Vec<u8>>: Unpin);
        assert_impl_all!(PinCursor<Vec<u8>>: Read, Write, Seek);
    }
}
