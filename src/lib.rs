use http::Uri;
use hyper::rt::{Read, ReadBuf, ReadBufCursor, Write};
use std::future;
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tower_service::Service;

#[derive(Clone)]
pub struct Connector<C, L> {
    inner: C,
    labels: L,
}

impl<C, L> Connector<C, L>
where
    C: Service<Uri>,
    L: Clone + FnOnce(&C::Response) -> Vec<metrics::Label>,
{
    pub fn new(inner: C, labels: L) -> Self {
        Self { inner, labels }
    }
}

impl<C, L> Service<Uri> for Connector<C, L>
where
    C: Service<Uri>,
    L: Clone + FnOnce(&C::Response) -> Vec<metrics::Label>,
{
    type Response = Stream<C::Response>;
    type Error = C::Error;
    type Future = Future<C::Future, L>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Uri) -> Self::Future {
        Future {
            inner: self.inner.call(request),
            labels: Some(self.labels.clone()),
        }
    }
}

#[pin_project::pin_project]
pub struct Future<F, L> {
    #[pin]
    inner: F,
    labels: Option<L>,
}

impl<F, S, E, L> future::Future for Future<F, L>
where
    F: future::Future<Output = Result<S, E>>,
    L: FnOnce(&S) -> Vec<metrics::Label>,
{
    type Output = Result<Stream<S>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx).map_ok(|inner| {
            let labels = this.labels.take().unwrap()(&inner);
            let new_counter = metrics::counter!("connections_established_total", labels.clone());
            let drop_counter = metrics::counter!("connections_closed_total", labels.clone());
            let read_counter = metrics::counter!("read_bytes", labels.clone());
            let write_counter = metrics::counter!("write_bytes", labels.clone());

            new_counter.increment(1);
            Stream {
                inner,
                drop_counter,
                read_counter,
                write_counter,
            }
        })
    }
}

#[pin_project::pin_project(PinnedDrop)]
pub struct Stream<S> {
    #[pin]
    inner: S,
    drop_counter: metrics::Counter,
    read_counter: metrics::Counter,
    write_counter: metrics::Counter,
}

#[pin_project::pinned_drop]
impl<S> PinnedDrop for Stream<S> {
    fn drop(self: Pin<&mut Self>) {
        self.drop_counter.increment(1)
    }
}

impl<S> Read for Stream<S>
where
    S: Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        let len = unsafe {
            let mut buf = ReadBuf::uninit(buf.as_mut());
            ready!(this.inner.poll_read(cx, buf.unfilled()))?;
            buf.filled().len()
        };
        unsafe { buf.advance(len) };
        this.read_counter.increment(len as _);
        Poll::Ready(Ok(()))
    }
}

impl<S> Write for Stream<S>
where
    S: Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        let len = ready!(this.inner.poll_write(cx, buf))?;
        this.write_counter.increment(len as _);
        Poll::Ready(Ok(len))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx)
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
        let len = ready!(this.inner.poll_write_vectored(cx, bufs))?;
        this.write_counter.increment(len as _);
        Poll::Ready(Ok(len))
    }
}

#[cfg(feature = "hyper-util")]
impl<S> hyper_util::client::legacy::connect::Connection for Stream<S>
where
    S: hyper_util::client::legacy::connect::Connection,
{
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        self.inner.connected()
    }
}
