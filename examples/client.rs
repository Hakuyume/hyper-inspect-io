use bytes::Bytes;
use http_body_util::Empty;
use hyper::Request;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use tower::ServiceExt;

#[tokio::main]
async fn main() {
    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(
        HttpConnector::new().map_response(|io| {
            hyper_inspect_io::Io::new(io, hyper_inspect_io::print_inspect::PrintInspect)
        }),
    );

    client
        .request(
            Request::builder()
                .uri("http://localhost:8080")
                .body(Empty::<Bytes>::new())
                .unwrap(),
        )
        .await
        .unwrap();
}
