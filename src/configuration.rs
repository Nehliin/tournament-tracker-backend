use config::{Config, Environment, File};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use tracing::error;

#[derive(Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    // DO NOT PRINT THIS IN LOGS!!
    pub private_key: String,
}
// DON'T DERIVE DEBUG TO AVOID ACCIDENTAL LOGGING!
#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };

        tracing::info!("Using Postgres SSL mode: {:?}", ssl_mode);

        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(&self.password)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = Config::default();

    let folder_path = std::env::current_dir().expect("Failed to figure out current directory");
    let config_dir = folder_path.join("configuration");

    // Read default config:
    settings.merge(File::from(config_dir.join("base")).required(true))?;

    // used to read ENVIROMENT variable, defaults to local
    let env = std::env::var("ENVIROMENT").unwrap_or_else(|_| "local".into());

    // Layer on the environment-specific values.
    settings.merge(config::File::from(config_dir.join(env)).required(true))?;

    // Allows ENV variables to override the yaml settings
    // ex APP_APPLICATION__PORT=1337
    settings.merge(Environment::with_prefix("app").separator("__"))?;

    let settings: Settings = settings.try_into()?;
    if settings.application.private_key.is_empty() {
        error!("Private key is not properly set!");
        Err(config::ConfigError::Message(
            "Private key isn't set".to_string(),
        ))
    } else {
        Ok(settings)
    }
}
