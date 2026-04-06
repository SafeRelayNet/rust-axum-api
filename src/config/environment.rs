// Environment configuration with layered loading and validation

use std::{borrow::Cow, collections::HashMap};
// * anyhow for convenient error handling
use anyhow::{Context, Result};
use tracing::warn;

/// Contains all environment variables used by the application
#[derive(Clone, Debug)]
pub struct EnvironmentVariables {
    pub environment: Cow<'static, str>,
    pub host: Cow<'static, str>,
    pub port: u16,
    pub protocol: Cow<'static, str>,
    pub max_request_body_size: usize,
    pub default_timeout_seconds: u64,
    pub db_host: Cow<'static, str>,
    pub db_port: u16,
    pub db_name: Cow<'static, str>,
    pub db_user: Cow<'static, str>,
    pub db_password: Cow<'static, str>,
    pub jwt_secret: Cow<'static, str>,
    pub jwt_exp_seconds: u64,
    pub redis_url: Cow<'static, str>,
}

impl EnvironmentVariables {
    /// Loads environment variables with priority: .env < .env.local < .env.production
    /// Always loads .env as base configuration, then overrides with environment-specific files
    pub fn load() -> Result<Self> {
        // Load base configuration from .env
        if let Err(e) = dotenv::from_path(".env") {
            warn!("Could not load .env file: {}", e);
        }

        // Check ENVIRONMENT to determine which additional file to load
        let environment: String =
            std::env::var("ENVIRONMENT").context("ENVIRONMENT variable is required")?;

        // Load environment-specific overrides
        match environment.as_str() {
            "production" => {
                if let Err(e) = dotenv::from_path(".env.production") {
                    warn!("Could not load .env.production file: {}", e);
                }
            }
            _ => {
                // Development: load .env.local for local overrides
                if let Err(e) = dotenv::from_path(".env.local") {
                    tracing::debug!("No .env.local file found: {}", e);
                }
            }
        }

        let vars: HashMap<String, String> = std::env::vars().collect();

        // Collect all missing variables instead of failing on the first one
        let mut missing_vars: Vec<String> = Vec::new();
        let mut parse_errors: Vec<String> = Vec::new();

        // Helper to check for required variables
        let check_var = |key: &str, missing_vars: &mut Vec<String>| -> Option<String> {
            match vars.get(key) {
                Some(value) => Some(value.clone()),
                None => {
                    missing_vars.push(key.to_string());
                    None
                }
            }
        };

        // Collect all variable values, tracking missing ones
        let host: Option<Cow<'static, str>> =
            check_var("HOST", &mut missing_vars).map(|s: String| Cow::<'static, str>::Owned(s));
        let port_str: Option<String> = check_var("PORT", &mut missing_vars);
        let protocol: Option<Cow<'static, str>> =
            check_var("PROTOCOL", &mut missing_vars).map(|s: String| Cow::<'static, str>::Owned(s));
        let max_body_size_str: Option<String> =
            check_var("MAX_REQUEST_BODY_SIZE", &mut missing_vars);
        let timeout_str: Option<String> = check_var("DEFAULT_TIMEOUT_SECONDS", &mut missing_vars);
        let db_host: Option<Cow<'static, str>> =
            check_var("DB_HOST", &mut missing_vars).map(|s: String| Cow::<'static, str>::Owned(s));
        let db_port_str: Option<String> = check_var("DB_PORT", &mut missing_vars);
        let db_name: Option<Cow<'static, str>> =
            check_var("DB_NAME", &mut missing_vars).map(|s: String| Cow::<'static, str>::Owned(s));
        let db_user: Option<Cow<'static, str>> =
            check_var("DB_USER", &mut missing_vars).map(|s: String| Cow::<'static, str>::Owned(s));
        let db_password: Option<Cow<'static, str>> = check_var("DB_PASSWORD", &mut missing_vars)
            .map(|s: String| Cow::<'static, str>::Owned(s));
        let jwt_secret: Option<Cow<'static, str>> = check_var("JWT_SECRET", &mut missing_vars)
            .map(|s: String| Cow::<'static, str>::Owned(s));
        let jwt_exp_seconds_str: Option<String> = check_var("JWT_EXP_SECONDS", &mut missing_vars);
        let redis_url: Option<Cow<'static, str>> = check_var("REDIS_URL", &mut missing_vars)
            .map(|s: String| Cow::<'static, str>::Owned(s));

        // Parse numeric values and collect format errors
        let port: Option<u16> = port_str.as_ref().and_then(|s: &String| {
            s.parse::<u16>()
                .map_err(|_| {
                    parse_errors.push(format!(
                        "PORT (current: \"{}\", should be: numeric value between 1-65535)",
                        s
                    ));
                })
                .ok()
        });

        let max_request_body_size: Option<usize> = max_body_size_str.as_ref().and_then(|s: &String| {
            s.parse::<usize>().map_err(|_| {
                parse_errors.push(format!("MAX_REQUEST_BODY_SIZE (current: \"{}\", should be: numeric value in bytes)", s));
            }).ok()
        });

        let default_timeout_seconds: Option<u64> = timeout_str.as_ref().and_then(|s: &String| {
            s.parse::<u64>().map_err(|_| {
                parse_errors.push(format!("DEFAULT_TIMEOUT_SECONDS (current: \"{}\", should be: numeric value in seconds)", s));
            }).ok()
        });

        let db_port: Option<u16> = db_port_str.as_ref().and_then(|s: &String| {
            s.parse::<u16>()
                .map_err(|_| {
                    parse_errors.push(format!(
                        "DB_PORT (current: \"{}\", should be: numeric value between 1-65535)",
                        s
                    ));
                })
                .ok()
        });

        let jwt_exp_seconds: Option<u64> = jwt_exp_seconds_str.as_ref().and_then(|s: &String| {
            s.parse::<u64>()
                .map_err(|_| {
                    parse_errors.push(format!(
                        "JWT_EXP_SECONDS (current: \"{}\", should be: numeric value in seconds)",
                        s
                    ));
                })
                .ok()
        });

        // Validate string variable formats
        if let Some(protocol_val) = &protocol {
            if !matches!(protocol_val.as_ref(), "http" | "https") {
                parse_errors.push(format!(
                    "PROTOCOL (current: \"{}\", should be: \"http\" or \"https\")",
                    protocol_val
                ));
            }
        }

        if !matches!(
            environment.as_str(),
            "development" | "staging" | "production"
        ) {
            parse_errors.push(format!("ENVIRONMENT (current: \"{}\", should be: \"development\", \"staging\", or \"production\")", environment));
        }

        // Generate comprehensive error message if any issues found
        if !missing_vars.is_empty() || !parse_errors.is_empty() {
            let mut error_msg: String = String::new();

            if !missing_vars.is_empty() {
                error_msg.push_str("\nMissing required environment variables:\n");
                for var in &missing_vars {
                    error_msg.push_str(&format!("  - {}\n", var));
                }
            }

            if !parse_errors.is_empty() {
                error_msg.push_str("Incorrect format environment variables:\n");
                for error in &parse_errors {
                    error_msg.push_str(&format!("  - {}\n", error));
                }
            }

            return Err(anyhow::anyhow!("{}", error_msg.trim_end()));
        }

        // Build EnvironmentVariables - the explicit extraction below keeps error paths typed and panic-free.
        let host_value: Cow<'static, str> =
            host.ok_or_else(|| anyhow::anyhow!("HOST missing after validation"))?;
        let port_value: u16 =
            port.ok_or_else(|| anyhow::anyhow!("PORT missing after validation"))?;
        let protocol_value: Cow<'static, str> =
            protocol.ok_or_else(|| anyhow::anyhow!("PROTOCOL missing after validation"))?;
        let max_request_body_size_value: usize = max_request_body_size
            .ok_or_else(|| anyhow::anyhow!("MAX_REQUEST_BODY_SIZE missing after validation"))?;
        let default_timeout_seconds_value: u64 = default_timeout_seconds
            .ok_or_else(|| anyhow::anyhow!("DEFAULT_TIMEOUT_SECONDS missing after validation"))?;
        let db_host_value: Cow<'static, str> =
            db_host.ok_or_else(|| anyhow::anyhow!("DB_HOST missing after validation"))?;
        let db_port_value: u16 =
            db_port.ok_or_else(|| anyhow::anyhow!("DB_PORT missing after validation"))?;
        let db_name_value: Cow<'static, str> =
            db_name.ok_or_else(|| anyhow::anyhow!("DB_NAME missing after validation"))?;
        let db_user_value: Cow<'static, str> =
            db_user.ok_or_else(|| anyhow::anyhow!("DB_USER missing after validation"))?;
        let db_password_value: Cow<'static, str> =
            db_password.ok_or_else(|| anyhow::anyhow!("DB_PASSWORD missing after validation"))?;
        let jwt_secret_value: Cow<'static, str> =
            jwt_secret.ok_or_else(|| anyhow::anyhow!("JWT_SECRET missing after validation"))?;
        let jwt_exp_seconds_value: u64 = jwt_exp_seconds
            .ok_or_else(|| anyhow::anyhow!("JWT_EXP_SECONDS missing after validation"))?;
        let redis_url_value: Cow<'static, str> =
            redis_url.ok_or_else(|| anyhow::anyhow!("REDIS_URL missing after validation"))?;

        Ok(Self {
            environment: Cow::Owned(environment),
            host: host_value,
            port: port_value,
            protocol: protocol_value,
            max_request_body_size: max_request_body_size_value,
            default_timeout_seconds: default_timeout_seconds_value,
            db_host: db_host_value,
            db_port: db_port_value,
            db_name: db_name_value,
            db_user: db_user_value,
            db_password: db_password_value,
            jwt_secret: jwt_secret_value,
            jwt_exp_seconds: jwt_exp_seconds_value,
            redis_url: redis_url_value,
        })
    }
}
