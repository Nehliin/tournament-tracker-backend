use config::{Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
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
    // TODO: merge this to set db settings in production
    // settings.merge(Environment::with_prefix("app"))?;

    // Layer on the environment-specific values.
    settings.merge(config::File::from(config_dir.join(env)).required(true))?;

    settings.try_into()
}
