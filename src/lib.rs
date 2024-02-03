use log::{Level, Record, LevelFilter, Metadata, Log};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use serde_json::Value;

#[macro_export]
macro_rules! structured_log {
    ($level:expr, $msg:expr, $log_type:expr, $data:expr, $tags:expr) => {{
        let structured_data = serde_json::json!({
            "type": $log_type,
            "data": $data,
            "tags": $tags,
        });

        match $level {
            log::Level::Error => log::error!("{} -- {}", $msg, structured_data),
            log::Level::Warn => log::warn!("{} -- {}", $msg, structured_data),
            log::Level::Info => log::info!("{} -- {}", $msg, structured_data),
            log::Level::Debug => log::debug!("{} -- {}", $msg, structured_data),
            log::Level::Trace => log::trace!("{} -- {}", $msg, structured_data),
            _ => log::info!("{} -- {}", $msg, structured_data),
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

struct ApiLogger {
    client: Client,
    api_url: String,
    auth_config: LogConfig,
    logger_config: LoggerConfig,
}

impl Log for ApiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let structured_data = serde_json::json!({
                "service": self.logger_config.service,
                "environment": self.logger_config.environment,
                "severity": record.level().to_string().to_lowercase(),
                "log": format!("{}", record.args()),
                // Additional data and tags would be included here if needed
            });

            let api_url = self.api_url.clone();
            tokio::spawn(async move {
                let client = Client::new();
                let _ = client.post(&api_url)
                    .json(&structured_data)
                    .send()
                    .await;
            });
        }
    }

    fn flush(&self) {}
}

impl ApiLogger {
    pub fn new(auth_config: LogConfig) -> ApiLogger {
        let api_url = env::var("LOG_API_URL").unwrap_or_else(|_| "http://your-default-api-endpoint.com".into());
        let logger_config = match &auth_config {
            LogConfig::ClientBuild { logger_config, .. } => logger_config.clone(),
            LogConfig::AccessKeys { logger_config, .. } => logger_config.clone(),
        };

        ApiLogger {
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
            let mut req = client.post(&api_url).json(&log_data);
            let _ = req.send().await;
        });
    }
}

pub fn init_logger(config: LogConfig) {
    let logger = ApiLogger::new(config);
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Failed to set logger");
}
