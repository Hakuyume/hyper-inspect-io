use hyper::rt::{Read, ReadBuf, ReadBufCursor, Write};
use std::cmp;
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub trait InspectRead {
    fn inspect_read(&mut self, _value: Result<&[u8], &io::Error>) {}
}

pub trait InspectWrite {
    fn inspect_write(&mut self, _value: Result<&[u8], &io::Error>) {}
    fn inspect_flush(&mut self, _value: Result<(), &io::Error>) {}
    fn inspect_shutdown(&mut self, _value: Result<(), &io::Error>) {}
    fn inspect_write_vectored<'a, I>(&mut self, value: Result<I, &io::Error>)
    where
        I: Iterator<Item = &'a [u8]>,
    {
        match value {
            Ok(bufs) => {
                for buf in bufs {
                    self.inspect_write(Ok(buf));
                }
            }
            Err(e) => self.inspect_write(Err(e)),
        }
    }
}

#[pin_project::pin_project]
#[derive(Clone, Debug)]
pub struct Io<T, I> {
    #[pin]
    inner: T,
    inspect: I,
}

impl<T, I> Io<T, I> {
    pub fn new(inner: T, inspect: I) -> Self {
        Self { inner, inspect }
    }
}

impl<T, I> Read for Io<T, I>
where
    T: Read,
    I: InspectRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        unsafe {
            let len = {
                let mut buf = ReadBuf::uninit(buf.as_mut());
                let value = ready!(this.inner.poll_read(cx, buf.unfilled()));
                this.inspect
                    .inspect_read(value.as_ref().map(|_| buf.filled()));
                value.map(|_| buf.filled().len())?
            };
            buf.advance(len);
        }
        Poll::Ready(Ok(()))
    }
}

impl<T, I> Write for Io<T, I>
where
    T: Write,
    I: InspectWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        this.inner.poll_write(cx, buf).map(|value| {
            this.inspect
                .inspect_write(value.as_ref().map(|len| &buf[..*len]));
            value
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map(|value| {
            this.inspect.inspect_flush(value.as_ref().map(|_| ()));
            value
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx).map(|value| {
            this.inspect.inspect_shutdown(value.as_ref().map(|_| ()));
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
            this.inspect
                .inspect_write_vectored(value.as_ref().map(|len| {
                    bufs.iter().scan(*len, |len, buf| {
                        let buf = &buf[..cmp::min(*len, buf.len())];
                        *len -= buf.len();
                        (!buf.is_empty()).then_some(buf)
                    })
                }));
            value
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

#[cfg(feature = "__examples")]
pub mod __examples;
#[cfg(test)]
mod tests;
