use log::{Level, Record, LevelFilter, Metadata, Log};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use serde_json::Value;

#[macro_export]
macro_rules! structured_log {
    ($level:expr, $msg:expr, $log_type:expr, $data:expr, $tags:expr) => {{
        // Encode the structured message as a JSON string
        let structured_message = serde_json::json!({
            "log": $msg,
            "type": $log_type,
            "data": $data,
            "tags": $tags,
        }).to_string();

        // Log the structured message as a JSON-encoded string
        match $level {
            log::Level::Error => log::error!("{}", structured_message),
            log::Level::Warn => log::warn!("{}", structured_message),
            log::Level::Info => log::info!("{}", structured_message),
            log::Level::Debug => log::debug!("{}", structured_message),
            log::Level::Trace => log::trace!("{}", structured_message),
            _ => log::info!("{}", structured_message),
        }
    }};
}

#[derive(Clone)]
pub enum LogConfig {
    ClientBuild { client_id: String, build_id: String, logger_config: LoggerConfig },
    AccessKeys { access_key: String, secret_key: String, logger_config: LoggerConfig },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub service: String,
    pub environment: String,
    pub default_type: Option<String>,
}

struct POGRLogger {
    client: Client,
    api_url: String,
    auth_config: LogConfig,
    logger_config: LoggerConfig,
}

impl Log for POGRLogger {
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let maybe_structured_message: Result<serde_json::Value, _> = serde_json::from_str(record.args().to_string().as_str());

            let mut structured_data = serde_json::json!({
                "service": self.logger_config.service,
                "environment": self.logger_config.environment,
                "severity": record.level().to_string().to_lowercase(),
            });

            if let Ok(mut structured_message) = maybe_structured_message {
                if structured_message.is_object() {
                    for (key, value) in structured_message.as_object_mut().unwrap().drain() {
                        structured_data[key] = value;
                    }
                }
            } else {
                structured_data["log"] = serde_json::Value::String(record.args().to_string());
            }

            let api_url = self.api_url.clone();
            let client = self.client.clone();
            let auth_config = self.auth_config.clone();

            tokio::spawn(async move {
                let mut req = client.post(&api_url).json(&structured_data);

                // Set headers based on auth_config
                match auth_config {
                    LogConfig::ClientBuild { client_id, build_id, .. } => {
                        req = req.header("POGR_CLIENT", client_id)
                                 .header("POGR_BUILD", build_id);
                    },
                    LogConfig::AccessKeys { access_key, secret_key, .. } => {
                        req = req.header("POGR_ACCESS", access_key)
                                 .header("POGR_SECRET", secret_key);
                    },
                }

                let _ = req.send().await;
            });
        }
    }

    fn flush(&self) {}
}



impl POGRLogger {
    pub fn new(auth_config: LogConfig) -> POGRLogger {
        let api_url = env::var("POGR_INTAKE_URL").unwrap_or_else(|_| "https://api.pogr.io/v1/intake/logs".into());
        let logger_config = match &auth_config {
            LogConfig::ClientBuild { logger_config, .. } => logger_config.clone(),
            LogConfig::AccessKeys { logger_config, .. } => logger_config.clone(),
        };

        POGRLogger {
            client: Client::new(),
            api_url,
            auth_config,
            logger_config,
        }
    }

    pub async fn custom_log(&self, level: Level, msg: &str, log_type: &str, data: Value, tags: Value) {
        let log_data = serde_json::json!({
            "service": self.logger_config.service,
            "environment": self.logger_config.environment,
            "severity": level.to_string().to_lowercase(),
            "type": log_type,
            "log": msg,
            "data": data,
            "tags": tags,
        });

        let api_url = self.api_url.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            tokio::spawn(async move {
                let mut req = client.post(&api_url).json(&log_data);

                // Set headers based on auth_config
                match auth_config {
                    LogConfig::ClientBuild { client_id, build_id, .. } => {
                        req = req.header("POGR_CLIENT", client_id)
                                 .header("POGR_BUILD", build_id);
                    },
                    LogConfig::AccessKeys { access_key, secret_key, .. } => {
                        req = req.header("POGR_ACCESS", access_key)
                                 .header("POGR_SECRET", secret_key);
                    },
                }

                let _ = req.send().await;
            });
        });
    }
}

pub fn init_logger(config: LogConfig) {
    let logger = POGRLogger::new(config);
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Failed to set logger");
}
