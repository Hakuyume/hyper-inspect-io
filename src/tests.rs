use bytes::Bytes;
use http::{Request, Response};
use http_body_util::{Empty, Full};
use hyper::body::Incoming;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::convert::Infallible;
use std::io;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceExt;

const BODY: &[u8] = b"hello world";

#[tokio::test]
async fn test_consistency() {
    let server_counter = Arc::new(Counter::default());
    let client_counter = Arc::new(Counter::default());

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    futures::future::join(
        async {
            async fn handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
                Ok(Response::builder()
                    .body(Full::new(Bytes::from_static(BODY)))
                    .unwrap())
            }

            let (stream, _) = listener.accept().await.unwrap();
            hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection(
                    crate::Io::new(TokioIo::new(stream), CountInspect(server_counter.clone())),
                    hyper::service::service_fn(handler),
                )
                .await
                .unwrap();
        },
        async {
            let counter = client_counter.clone();
            let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(
                HttpConnector::new()
                    .map_response(move |io| crate::Io::new(io, CountInspect(counter.clone()))),
            );
            client
                .request(
                    Request::builder()
                        .uri(format!("http://{local_addr}"))
                        .body(Empty::<Bytes>::new())
                        .unwrap(),
                )
                .await
                .unwrap();
        },
    )
    .await;

    assert!(server_counter.read.load(Ordering::SeqCst) > 0);
    assert!(server_counter.write.load(Ordering::SeqCst) > BODY.len());
    assert!(client_counter.read.load(Ordering::SeqCst) > BODY.len());
    assert!(client_counter.write.load(Ordering::SeqCst) > 0);
    assert_eq!(
        server_counter.read.load(Ordering::SeqCst),
        client_counter.write.load(Ordering::SeqCst),
    );
    assert_eq!(
        server_counter.write.load(Ordering::SeqCst),
        client_counter.read.load(Ordering::SeqCst),
    );
}

#[derive(Default)]
struct Counter {
    read: AtomicUsize,
    write: AtomicUsize,
}

struct CountInspect(Arc<Counter>);
impl crate::InspectRead for CountInspect {
    fn inspect_read(&mut self, value: Result<&[u8], &io::Error>) {
        if let Ok(value) = value {
            self.0.read.fetch_add(value.len(), Ordering::SeqCst);
        }
    }
}
impl crate::InspectWrite for CountInspect {
    fn inspect_write(&mut self, value: Result<&[u8], &io::Error>) {
        if let Ok(value) = value {
            self.0.write.fetch_add(value.len(), Ordering::SeqCst);
        }
    }
}
