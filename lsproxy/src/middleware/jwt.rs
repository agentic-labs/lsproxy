use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use futures_util::future::{ready, Ready};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;

pub fn is_auth_enabled() -> bool {
    env::var("USE_AUTH").map(|v| v == "true").unwrap_or(true)
}

pub fn validate_jwt_config() -> Result<String, String> {
    if !is_auth_enabled() {
        return Ok("Authentication disabled".to_string());
    }

    match env::var("JWT_SECRET") {
        Ok(secret) => Ok(secret),
        Err(_) => Err("JWT_SECRET environment variable not set. To use authentication you need to set this variable. If you want to turn off authentication you can set USE_AUTH=false.".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
}

pub struct JwtMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService { service }))
    }
}

pub struct JwtMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get("Authorization");

        if let Some(auth_header) = auth_header {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");
                    let secret = match std::env::var("JWT_SECRET") {
                        Ok(secret) => secret,
                        Err(_) => {
                            return Box::pin(async move {
                                Err(actix_web::error::ErrorInternalServerError(
                                    "JWT_SECRET environment variable not set",
                                ))
                            });
                        }
                    };

                    match decode::<Claims>(
                        token,
                        &DecodingKey::from_secret(secret.as_bytes()),
                        &Validation::default(),
                    ) {
                        Ok(_) => {
                            let fut = self.service.call(req);
                            return Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            });
                        }
                        Err(_) => {
                            return Box::pin(async move {
                                Err(actix_web::error::ErrorUnauthorized("Invalid token"))
                            });
                        }
                    }
                }
            }
        }

        Box::pin(async move {
            Err(actix_web::error::ErrorUnauthorized(
                "Missing or invalid authorization header",
            ))
        })
    }
}
