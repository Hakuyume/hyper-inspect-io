#[doc(hidden)]
pub mod print_inspect;

use hyper::rt::{Read, ReadBuf, ReadBufCursor, Write};
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub trait Inspect {
    fn read(&mut self, _: &io::Result<&[u8]>) {}
    fn write(&mut self, _: &io::Result<&[u8]>) {}
    fn flush(&mut self, _: &io::Result<()>) {}
    fn shutdown(&mut self, _: &io::Result<()>) {}
    fn write_vectored(&mut self, _: &io::Result<(&[IoSlice<'_>], usize)>) {}
}

#[pin_project::pin_project]
#[derive(Clone, Debug)]
pub struct Io<T, I> {
    #[pin]
    inner: T,
    inspect: I,
}

impl<T, I> Io<T, I> {
    pub fn new(inner: T, inspect: I) -> Self
    where
        I: Inspect,
    {
        Self { inner, inspect }
    }
}

impl<T, I> Read for Io<T, I>
where
    T: Read,
    I: Inspect,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        unsafe {
            let value = {
                let mut buf = ReadBuf::uninit(buf.as_mut());
                let value = ready!(this.inner.poll_read(cx, buf.unfilled()));
                let value = value.map(|_| buf.filled());
                this.inspect.read(&value);
                value.map(<[_]>::len)
            };
            match value {
                Ok(len) => {
                    buf.advance(len);
                    Poll::Ready(Ok(()))
                }
                Err(e) => Poll::Ready(Err(e)),
            }
        }
    }
}

impl<T, I> Write for Io<T, I>
where
    T: Write,
    I: Inspect,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        this.inner.poll_write(cx, buf).map(|value| {
            let value = value.map(|len| &buf[..len]);
            this.inspect.write(&value);
            value.map(<[_]>::len)
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map(|value| {
            this.inspect.flush(&value);
            value
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx).map(|value| {
            this.inspect.shutdown(&value);
            value
        })
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        this.inner.poll_write_vectored(cx, bufs).map(|value| {
            let value = value.map(|len| (bufs, len));
            this.inspect.write_vectored(&value);
            value.map(|(_, len)| len)
        })
    }
}

#[cfg(feature = "hyper-util")]
impl<T, I> hyper_util::client::legacy::connect::Connection for Io<T, I>
where
    T: hyper_util::client::legacy::connect::Connection,
{
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        self.inner.connected()
    }
}
