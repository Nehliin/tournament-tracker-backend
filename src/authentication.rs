use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use actix_web::ResponseError;
use futures::future::{ok, Either, Ready};
use futures::task::{Context, Poll};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub struct Authentication {
    pool: PgPool,
}

impl<S, B> Transform<S> for Authentication
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware {
            service,
            pool: self.pool.clone(),
        })
    }
}

const AUTHORIZATION_HEADER: &str = "Authorization";
const BEARER_PREFIX: &str = "Bearer ";
const TEMP_SECRET: &[u8] = &vec![0; 512];

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    uuid: String, // maybe something else than this
    sub: String,  //add later maybe
}

pub struct AuthenticationMiddleware<S> {
    service: S,
    pool: PgPool,
}

impl<S> AuthenticationMiddleware<S> {
    async fn is_authorized_request(&self, token: &str) -> bool {
        // 1. validate token
        // check session uuid in db? or just the token itself?
        let validation = Validation {
            sub: Some(String::from("user agent header")),
            ..Validation::default()
        };
        if let Ok(token_claims) =
            decode::<Claims>(token, &DecodingKey::from_secret(TEMP_SECRET), &validation)
        {
            // check authentication
            true
        } else {
            false
        }
    }
}

// TODO: create a UserStruct that implements FromRequest which this can create and append to request
// which the handlers can use to get the logged in userinfo
impl<S, B> Service for AuthenticationMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error; //ServerError?
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        if let Some(header_value) = req.headers().get(AUTHORIZATION_HEADER) {
            if let Some(token) = header_value
                .to_str()
                .map(|str| str.strip_prefix(BEARER_PREFIX))
                .ok()
                .flatten()
            {}
        }
        todo!()
    }
}
