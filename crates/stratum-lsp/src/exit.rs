use std::task::{Context, Poll};

use tower::Service;
use tower_lsp::jsonrpc::{Request, Response};

#[derive(Debug)]
pub struct ExitOnNotification<S> {
    inner: S,
    shutdown_requested: bool,
}

impl<S> ExitOnNotification<S> {
    pub const fn new(inner: S) -> Self {
        Self {
            inner,
            shutdown_requested: false,
        }
    }
}

impl<S> Service<Request> for ExitOnNotification<S>
where
    S: Service<Request, Response = Option<Response>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req.method() {
            "shutdown" => self.shutdown_requested = true,
            "exit" => std::process::exit(i32::from(!self.shutdown_requested)),
            _ => {}
        }
        self.inner.call(req)
    }
}
