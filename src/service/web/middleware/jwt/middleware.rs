use std::rc::Rc;

use actix_utils::future::ok;
use actix_web::{
    Error, HttpMessage,
    body::{EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, forward_ready},
};
use futures::future::{FutureExt as _, LocalBoxFuture};

use crate::service::web::middleware::jwt::inner::Inner;

pub struct JwtMiddleware<S> {
    pub(crate) service: S,
    pub(crate) inner: Rc<Inner>,
}

impl<S, B> Service<ServiceRequest> for JwtMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 从Header提取Token
        let auth_header = req.headers().get("Authorization");
        let token = match auth_header {
            Some(header) => {
                let header_str = header.to_str().unwrap_or("");
                if header_str.starts_with("Bearer ") {
                    header_str["Bearer ".len()..].to_string()
                } else {
                    "".to_string()
                }
            }
            None => "".to_string(),
        };

        // 验证Token
        if token.is_empty() {
            // 如果没有Token，则返回401错误
            let res = req.error_response(crate::service::errors::Error::MissingToken);
            return ok(res.map_into_right_body()).boxed_local();
        }

        match self.inner.validate(token.as_str()) {
            Ok(claims) => {
                // 将用户信息存入请求扩展，供后续处理使用
                req.extensions_mut().insert(claims);
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Err(e) => {
                // 如果Token验证失败，则返回401错误
                let res = req.error_response(e);
                ok(res.map_into_right_body()).boxed_local()
            }
        }
    }
}
