use std::rc::Rc;
use std::pin::Pin;
use std::cell::RefCell;
use std::task::{Context, Poll};

use futures::Future;
use futures::future::{ok, Ready};
use actix_service::{Service, Transform};
use actix_web::{Error, HttpMessage};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_http::body::MessageBody;

// custom request auth middleware
pub struct Auth;

impl<S, B> Transform<S> for Auth
    where
        S: Service<Request=ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(RefCell::new(service))
        })
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for AuthMiddleware<S>
    where
        S: Service<Request=ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let mut svc = self.service.clone();

        Box::pin(async move {
            let user_id = match req.headers().get("user_id") {
                Some(v) => v.to_str().unwrap_or_default().to_string().parse().unwrap_or_default(),
                None => 0,
            };

            let auth_key = match req.headers().get("token") {
                Some(v) => v.to_str().unwrap_or_default().to_string(),
                None => "".to_string(),
            };

            let mut ctx = crate::utility::context::Context::new();
            ctx.insert("user_id", user_id);
            ctx.insert("auth_key", auth_key);
            req.extensions_mut().insert(ctx);

            let resp = svc.call(req).await?;

            Ok(resp)
        })
    }
}
