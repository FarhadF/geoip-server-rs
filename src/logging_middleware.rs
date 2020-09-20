use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, Ready};
use slog::info;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Logging {
    logger: slog::Logger,
}

impl Logging {
    pub fn new(logger: slog::Logger) -> Logging {
        Logging { logger }
    }
}

impl<S, B> Transform<S> for Logging
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = LoggingMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggingMiddleware {
            service,
            logger: self.logger.clone(),
        })
    }
}

pub struct LoggingMiddleware<S> {
    service: S,
    logger: slog::Logger,
}

impl<S, B> Service for LoggingMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let start_time = chrono::Utc::now();
        let logger = self.logger.clone();
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            let req = res.request();
            let end_time = chrono::Utc::now();
            let duration = end_time - start_time;
            info!(logger, "handled request";
            "responseTime" => duration.num_nanoseconds(),
            "url" => %req.uri(),
            "route" => req.path(),
            "method" => %req.method(),
            "statusCode" => res.status().as_u16()
            );
            Ok(res)
        })
    }
}
