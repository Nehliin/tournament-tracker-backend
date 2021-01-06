use crate::configuration::Settings;
use crate::{stores::user_store::UserStore, ServerError};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::{dev::ServiceRequest, Error, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::Local;
use futures::future::{err, ready};
use futures::prelude::future::Ready;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{error, warn};
use uuid::Uuid;

const AUTH_HEADER: &str = "Authorization";
const HEADER_PREFIX: &str = "Bearer ";
const THREE_DAYS_SECONDS: usize = 60 * 60 * 24 * 3;
const PATTERN: &str = include_str!("../email_regex.txt");
static DECODING_KEY: OnceCell<DecodingKey> = OnceCell::new();
static ENCODING_KEY: OnceCell<EncodingKey> = OnceCell::new();
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(PATTERN).expect("Regex is invalid"));

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

#[derive(Debug)]
pub struct UserInfo {
    pub id: Uuid,
}

impl FromRequest for UserInfo {
    type Error = ServerError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload<PayloadStream>) -> Self::Future {
        if let Some(auth_header) = req.headers().get(AUTH_HEADER) {
            match auth_header.to_str() {
                Ok(header_str) => {
                    if let Some(token) = header_str.strip_prefix(HEADER_PREFIX) {
                        ready(decode_token(token))
                    } else {
                        err(ServerError::InvalidToken(
                            "Missing Bearer prefix".to_string(),
                        ))
                    }
                }
                Err(_) => err(ServerError::InvalidToken(
                    "Invalid header encoding".to_string(),
                )),
            }
        } else {
            err(ServerError::InvalidToken(
                "Missing Authorization header".to_string(),
            ))
        }
    }
}

pub fn set_keys(config: &Settings) {
    let key = &config.application.private_key;
    assert!(48 <= key.len(), "Private key is too short");
    DECODING_KEY
        .set(DecodingKey::from_base64_secret(key).expect("Key must be base64 encoded"))
        .expect("Failed to set decoding key");
    ENCODING_KEY
        .set(EncodingKey::from_base64_secret(key).expect("Key must be base64 encoded"))
        .expect("Failed to set encoding key");
}

fn decode_token(token: &str) -> Result<UserInfo, ServerError> {
    let validation = Validation {
        leeway: 3,
        ..Validation::default()
    };

    let decoded_token = decode::<Claims>(
        token,
        &DECODING_KEY.get().expect("Decoding key hasn't been set"),
        &validation,
    )
    .map_err(|err| {
        warn!("Token decoding error: {}", err);
        ServerError::InvalidToken("Invalid token".to_string())
    })?;

    let uuid = Uuid::parse_str(&decoded_token.claims.sub).map_err(|err| {
        error!(
            "invalid uuid string ({}), error: {}",
            &decoded_token.claims.sub, err
        );
        ServerError::InvalidToken("Invalid token content".to_string())
    })?;
    Ok(UserInfo { id: uuid })
}

// Authenticate the request given an auth token
pub async fn authenticate_request(
    pool: PgPool,
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    let token = credentials.token();
    let uuid = decode_token(token)?.id;
    if pool.get_user(uuid).await.is_some() {
        Ok(req)
    } else {
        Err(ServerError::InvalidToken("User no longer exists".to_string()).into())
    }
}

// Authenticate an user and return a JWT token if the credentials are valid
pub async fn login_user(
    storage: &PgPool,
    email: &str,
    password: &str,
) -> Result<String, ServerError> {
    // Is this really needed? Gets rid of unnecessary db call at least
    if !EMAIL_REGEX.is_match(email) {
        return Err(ServerError::InvalidEmail);
    }
    if let Some(user_row) = storage.find_user(email).await {
        // check password
        let is_pw_correct = bcrypt::verify(&password, &user_row.password).map_err(|err| {
            error!("Failed to do password verification: {}", err);
            ServerError::InvalidPassword
        })?;

        if !is_pw_correct {
            return Err(ServerError::InvalidPassword);
        }
        let current_unix_time = Local::now().timestamp() as usize;
        // create token
        let claims = Claims {
            exp: current_unix_time + THREE_DAYS_SECONDS,
            iat: current_unix_time,
            sub: user_row.id.to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &ENCODING_KEY.get().expect("Encoding key hasn't been set"),
        )
        .map_err(|err| {
            error!("Failed to encode JWT token: {}", err);
            ServerError::LoginFailed
        })?;
        Ok(token)
    } else {
        Err(ServerError::InvalidEmail)
    }
}

pub async fn create_user(
    storage: &PgPool,
    email: &str,
    password: &str,
) -> Result<Uuid, ServerError> {
    if !EMAIL_REGEX.is_match(email) {
        return Err(ServerError::InvalidEmail);
    }

    if password.len() < 8 {
        return Err(ServerError::InvalidPassword);
    }

    if storage.find_user(email).await.is_some() {
        return Err(ServerError::AccountAlreadyExists(email.to_string()));
    }
    let id = storage.insert_user(email, password).await?;
    Ok(id)
}
