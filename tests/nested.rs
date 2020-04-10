use futures::future::BoxFuture;
use hyper::{Body, body};
use tide::{Request, Endpoint};
use std::sync::Arc;

#[tokio::test]
async fn nested() {
    let mut inner = tide::new();
    inner.at("/foo").get(|_| async { "foo" });
    inner.at("/bar").get(|_| async { "bar" });

    let mut outer = tide::new();
    // Nest the inner app on /foo
    outer.at("/foo").nest(inner);

    let app = outer.into_http_service();

    let req = hyper::Request::get("/foo/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"foo");

    let req = hyper::Request::get("/foo/bar").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"bar");
}

#[tokio::test]
async fn nested_middleware() {
    let echo_path = |req: tide::Request<()>| async move { req.uri().path().to_string() };
    fn test_middleware(
        req: tide::Request<()>,
        next: tide::Next<'_, ()>,
    ) -> BoxFuture<'_, tide::Response> {
        Box::pin(async move {
            let res = next.run(req).await;
            res.set_header("X-Tide-Test", "1")
        })
    }

    let mut app = tide::new();

    let mut inner_app = tide::new();
    inner_app.middleware(test_middleware);
    inner_app.at("/echo").get(echo_path);
    inner_app.at("/:foo/bar").strip_prefix().get(echo_path);
    app.at("/foo").nest(inner_app);

    app.at("/bar").get(echo_path);

    let app = app.into_http_service();

    let req = hyper::Request::get("/foo/echo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(
        res.headers().get("X-Tide-Test"),
        Some(&"1".parse().unwrap())
    );
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"/echo");

    let req = hyper::Request::get("/foo/x/bar").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(
        res.headers().get("X-Tide-Test"),
        Some(&"1".parse().unwrap())
    );
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"/");

    let req = hyper::Request::get("/bar").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(res.headers().get("X-Tide-Test"), None);
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"/bar");
}

#[tokio::test]
async fn nested_with_different_state() {
    let mut outer = tide::new();
    let mut inner = tide::with_state(42);
    inner.at("/").get(|req: tide::Request<i32>| async move {
        let num = req.state();
        format!("the number is {}", num)
    });
    outer.at("/").get(|_| async move { "Hello, world!" });
    outer.at("/foo").nest(inner);

    let app = outer.into_http_service();

    let req = hyper::Request::get("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"the number is 42");

    let req = hyper::Request::get("/").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = app.call(req).await;
    assert_eq!(res.status(), 200);
    let buf = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&buf[..], &*b"Hello, world!");
}
