use bytes::Bytes;
use http::{Request, Response};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::convert::Infallible;
use std::net::Ipv4Addr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 8080))
        .await
        .unwrap();
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let _ = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
            .serve_connection(
                hyper_inspect_io::Io::new(
                    TokioIo::new(stream),
                    hyper_inspect_io::print_inspect::PrintInspect,
                ),
                hyper::service::service_fn(handler),
            )
            .await;
    }
}

async fn handler(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::builder()
        .body(Full::new(Bytes::from("hello world")))
        .unwrap())
}
