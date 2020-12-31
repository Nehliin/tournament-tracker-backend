use crate::{stores::user_store::UserStore, ServerError};
use actix_web::{dev::ServiceRequest, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::Local;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{error, warn};
use uuid::Uuid;

const TEMP_SECRET: &[u8] = &[0; 512];
const THREE_DAYS_SECONDS: usize = 60 * 60 * 24 * 3;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

// Authenticate the request given an auth token
pub async fn authenticate_request(
    pool: PgPool,
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    let token = credentials.token();
    let validation = Validation {
        leeway: 3,
        ..Validation::default()
    };

    let decoded_token =
        decode::<Claims>(token, &DecodingKey::from_secret(TEMP_SECRET), &validation).map_err(
            |err| {
                warn!("Token decoding error: {}", err);
                ServerError::InvalidToken
            },
        )?;

    let uuid = Uuid::parse_str(&decoded_token.claims.sub).map_err(|err| {
        error!(
            "invalid uuid string ({}), error: {}",
            &decoded_token.claims.sub, err
        );
        ServerError::InvalidToken
    })?;

    if pool.get_user(uuid).await.is_some() {
        Ok(req)
    } else {
        Err(ServerError::InvalidToken.into())
    }
}

const PATTERN: &str = include_str!("../email_regex.txt");
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(PATTERN).expect("Regex is invalid"));

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
            &EncodingKey::from_secret(TEMP_SECRET),
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

pub async fn create_user(storage: &PgPool, email: &str, password: &str) -> Result<(), ServerError> {
    if !EMAIL_REGEX.is_match(email) {
        return Err(ServerError::InvalidEmail);
    }

    if password.len() < 8 {
        ret
    }
}
