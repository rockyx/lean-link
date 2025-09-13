use actix_web::{
    Error,
    body::{EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use futures::future::{Ready, ok};
use jsonwebtoken::Algorithm;
use std::rc::Rc;

use crate::service::web::middleware::jwt::{inner::Inner, middleware::JwtMiddleware};

pub struct Jwt {
    inner: Rc<Inner>,
}

impl Jwt {
    pub fn new(secret_key: String, algorithm: Algorithm) -> Self {
        let inner = Rc::new(Inner::new(secret_key, algorithm));

        Self { inner }
    }

    pub fn set_secret_key(mut self, secret_key: String) -> Self {
        Rc::make_mut(&mut self.inner).secret_key = secret_key;
        self
    }

    pub fn set_algorithm(mut self, algorithm: Algorithm) -> Self {
        Rc::make_mut(&mut self.inner).algorithm = algorithm;
        self
    }
}

impl Default for Jwt {
    fn default() -> Self {
        let inner = Rc::new(Inner::default());

        Self { inner }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Jwt
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(JwtMiddleware {
            service: service,
            inner: self.inner.clone(),
        })
    }
}
