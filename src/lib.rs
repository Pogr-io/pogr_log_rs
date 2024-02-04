//! A custom logging module utilizing `log` and `reqwest` for structured logging with remote log aggregation capabilities.


use log::{set_logger, set_max_level, Level, Record, LevelFilter, Metadata, Log};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use serde_json::Value;
use std::sync::Mutex;
use once_cell::sync::OnceCell;

/// Structured logging macro for easy logging of structured data.
///
/// # Parameters
/// - `$level`: The log level (e.g., `log::Level::Info`).
/// - `$msg`: The log message as a string.
/// - `$log_type`: A string representing the type of log (e.g., "error", "request").
/// - `$data`: JSON serializable data associated with the log.
/// - `$tags`: JSON serializable tags for categorizing the log.
///
/// # Examples
/// ```
/// structured_log!(log::Level::Info, "User logged in", "login", {"user_id": 123}, {"env": "production"});
/// ```
#[macro_export]
macro_rules! structured_log {
    // Define the macro with parameters for log level, message, log type, data, and tags.
    ($level:expr, $msg:expr, $log_type:expr, $data:expr, $tags:expr) => {{
        // Use `serde_json::json!` macro to encode the structured message as a JSON object,
        // including the log message, type, data, and tags.
        let structured_message = serde_json::json!({
            "log": $msg,
            "type": $log_type,
            "data": $data,
            "tags": $tags,
        }).to_string(); // Convert the JSON object to a string for logging.

        // Match the provided log level and call the corresponding logging function from the `log` crate.
        // The `log` crate provides a macro for each log level (error, warn, info, debug, trace).
        // The structured message is logged as a JSON-encoded string.
        match $level {
            log::Level::Error => log::error!("{}", structured_message),
            log::Level::Warn => log::warn!("{}", structured_message),
            log::Level::Info => log::info!("{}", structured_message),
            log::Level::Debug => log::debug!("{}", structured_message),
            log::Level::Trace => log::trace!("{}", structured_message),
            // This catch-all arm is technically unnecessary because all possible values of `log::Level`
            // are already covered. It's a good practice to handle the default case, but here it could
            // lead to unexpected behaviors if the `log::Level` enum is extended in the future.
            // It's safe to remove this arm, ensuring all log messages are handled explicitly by their level.
            _ => log::info!("{}", structured_message),
        }
    }};
}


/// Represents the configuration for logging, supporting different authentication methods.
#[derive(Clone)]
pub enum LogConfig {
    /// Configuration for client-based authentication.
    ClientBuild { client_id: String, build_id: String, logger_config: LoggerConfig },
    /// Configuration for API key-based authentication.
    AccessKeys { access_key: String, secret_key: String, logger_config: LoggerConfig },
}

/// Configuration for the logger itself, including service and environment identifiers.
#[derive(Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub service: String,
    pub environment: String,
    pub default_type: Option<String>,
}


/// A logger implementation that sends logs to a remote server.
///
/// Utilizes `reqwest` for HTTP requests, and supports structured logging through JSON serialization.
pub struct POGRLogger {
    client: Option<Client>,
    api_url: Option<String>,
    auth_config: LogConfig,
    logger_config: LoggerConfig,
}

/// Implements the `Log` trait for the `POGRLogger` struct, enabling structured and asynchronous logging.
///
/// `POGRLogger` is designed to send log messages as structured JSON data to a remote logging service.
/// It enriches log records with additional metadata such as service name, environment, and severity level
/// before asynchronously posting them to a specified API endpoint. This implementation supports dynamic
/// log level filtering, structured log data parsing, and configurable authentication for secure log transmission.
impl Log for POGRLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Check if the log level of the record is enabled in this logger's configuration.
        // This is a simplified example. You should adjust the logic to match your logger's
        // configuration and how it determines which log levels to enable.
        metadata.level() <= log::Level::Info
    }
    /// Logs a record.
    ///
    /// This method is invoked for every log message that is enabled by the current log level configuration.
    /// It checks if the log level of the incoming record is enabled and, if so, proceeds to process the log message.
    ///
    /// # Arguments
    /// * `record` - A reference to the log `Record` that contains the log message and metadata.
    ///
    /// # Behavior
    /// - Attempts to parse the log message as a JSON string. If successful and the parsed message is a JSON object,
    ///   it merges the JSON fields into the structured log data. Otherwise, it includes the original log message as a string.
    /// - Prepares structured log data with default fields (`service`, `environment`, `severity`) and any fields extracted
    ///   from the structured log message.
    /// - Asynchronously sends the structured log data to a configured remote API endpoint, using a cloned HTTP client
    ///   and applying authentication headers based on the logger's configuration.
    fn log(&self, record: &Record) {
        // Checks if the log level of the record is enabled for this logger.
        if self.enabled(record.metadata()) {
            // Attempts to parse the log message as structured JSON data.
            let maybe_structured_message: Result<serde_json::Value, _> = serde_json::from_str(record.args().to_string().as_str());

            // Initializes structured data with default fields: service, environment, and severity.
            let mut structured_data = serde_json::json!({
                "service": self.logger_config.service,
                "environment": self.logger_config.environment,
                "severity": record.level().to_string().to_lowercase(),
            });

            // If the log message is valid JSON and is an object, merge its fields into `structured_data`.
            if let Ok(mut structured_message) = maybe_structured_message {
                if let serde_json::Value::Object(ref mut obj) = structured_message {
                    // Take the map out and replace it with an empty map
                    let drained_map = std::mem::take(obj);

                    // Iterate over all fields in the drained map and add them to `structured_data`.
                    for (key, value) in drained_map {
                        structured_data[&key] = value;
                    }
                }
            } else {
                // If the log message isn't structured JSON, include it as a plain string under the "log" key.
                structured_data["log"] = serde_json::Value::String(record.args().to_string());
            }


            // Clone necessary data for the asynchronous context.
            let api_url = self.api_url.clone().expect("API URL must be set");
            let client = self.client.clone();
            let auth_config = self.auth_config.clone();

            // Spawn an asynchronous task to send the log data to a remote API.
            tokio::spawn(async move {
                // Prepare the HTTP request with the structured log data as JSON.
                let mut req = client.expect("REASON").post(&api_url).json(&structured_data);

                // Set request headers based on the authentication configuration.
                match auth_config {
                    LogConfig::ClientBuild { client_id, build_id, .. } => {
                        // If using client/build ID for auth, set headers accordingly.
                        req = req.header("POGR_CLIENT", client_id)
                                 .header("POGR_BUILD", build_id);
                    },
                    LogConfig::AccessKeys { access_key, secret_key, .. } => {
                        // If using access/secret keys for auth, set headers accordingly.
                        req = req.header("POGR_ACCESS", access_key)
                                 .header("POGR_SECRET", secret_key);
                    },
                }

                // Send the request. The result is ignored with `_` since we don't handle response or errors here.
                let _ = req.send().await;
            });
        }
    }

    /// Flushes buffered log records.
    ///
    /// This implementation of `flush` does not perform any action because `POGRLogger` sends each log record
    /// asynchronously upon creation, leaving no buffered records to flush. This method is required by the `Log` trait
    /// but can be left empty in cases like this where immediate or asynchronous log handling is used.
    ///
    /// # Examples
    /// This method would be called by the logging framework or manually to ensure that all buffered logs are
    /// flushed to their destination, typically during application shutdown or after a critical error to ensure
    /// all relevant information is logged. Since `POGRLogger` does not buffer logs, calling this method has no effect.
    fn flush(&self) {}
}




impl POGRLogger {
    /// Constructs a new `POGRLogger` instance with the provided authentication configuration.
    ///
    /// This method initializes the logger with necessary configurations for authenticating
    /// and sending logs to a remote logging service. It determines the API URL from an environment
    /// variable or uses a default URL if the environment variable is not set.
    ///
    /// # Parameters
    /// - `auth_config`: Authentication configuration which can vary based on the method of authentication
    ///   (e.g., client/build ID or access/secret keys).
    ///
    /// # Returns
    /// A new instance of `POGRLogger` configured with the specified authentication method and API URL.
    pub fn new(client: Client, api_url: Option<String>, auth_config: LogConfig, logger_config: LoggerConfig) -> Self {
        // Attempts to retrieve the API URL from an environment variable, defaults to a predefined URL if not found.
        let api_url = if let Some(url) = api_url {
            url
        } else {
            // If `api_url` is not provided, try retrieving the URL from an environment variable.
            match env::var("POGR_INTAKE_URL") {
                Ok(url) => url, // Use environment variable if set
                Err(_) => "https://api.pogr.io/v1/intake/logs".to_string(), // Default URL if env var is not set
            }
        };
        //println!("POGR server URL: {}", api_url); // Log the URL to the console

        // Constructs the `POGRLogger` instance with the resolved configurations.
        POGRLogger {
            client: Some(client), // Initializes a new HTTP client for sending requests.
            api_url: Some(api_url), // The determined API URL for log intake.
            auth_config, // The provided authentication configuration.
            logger_config, // The determined logger configuration.
        }
    }

    pub fn set_client(&mut self, client: Client) {
        self.client = Some(client);
    }

    pub fn set_api_url(&mut self, api_url: String) {
        self.api_url = Some(api_url);
    }

    /// Asynchronously sends a custom log message to the remote server.
    ///
    /// Allows for detailed customization of the log message by specifying log level, message,
    /// log type, data, and tags. The log data is structured and sent as a JSON object.
    ///
    /// # Parameters
    /// - `level`: The severity level of the log message.
    /// - `msg`: The log message text.
    /// - `log_type`: A string representing the type of log (e.g., "error", "transaction").
    /// - `data`: Additional structured data to include with the log message.
    /// - `tags`: Tags for categorizing or filtering log messages.
    ///
    /// # Notes
    /// This method spawns an asynchronous task to send the log data, ensuring that logging
    /// does not block the main execution flow of the application.
    #[allow(dead_code)]
    pub async fn custom_log(&self, level: Level, msg: &str, log_type: &str, data: Value, tags: Value) {
        // Check if the client is initialized. In this context, we assume the client should always be Some.
        // If this is not the case, you might need to revisit where and how `self.client` is initialized.
        let client = match self.client.clone() {
            Some(client) => client,
            None => {
                eprintln!("HTTP client is not initialized.");
                return;
            }
        };
    
        let api_url = self.api_url.clone().unwrap_or_else(|| {
            eprintln!("API URL is not set, using default.");
            "https://api.pogr.io/v1/intake/logs".to_string()
        });
    
        let auth_config = self.auth_config.clone();
    
        let log_data = serde_json::json!({
            "service": self.logger_config.service,
            "environment": self.logger_config.environment,
            "severity": level.to_string().to_lowercase(),
            "type": log_type,
            "log": msg,
            "data": data,
            "tags": tags,
        });
    
        tokio::spawn(async move {
            let req = client.post(&api_url)
                .json(&log_data)
                .header("content-type", "application/json");
    
            let req = match auth_config {
                LogConfig::ClientBuild { client_id, build_id, .. } => {
                    req.header("POGR_CLIENT", &client_id)
                        .header("POGR_BUILD", &build_id)
                },
                LogConfig::AccessKeys { access_key, secret_key, .. } => {
                    req.header("POGR_ACCESS", &access_key)
                        .header("POGR_SECRET", &secret_key)
                },
            };
    
            match req.send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        eprintln!("Failed to send log data, HTTP Error: {}", response.status());
                    }
                },
                Err(e) => eprintln!("Failed to send log data: {}", e),
            }
        });
    }
    
       
}



static LOGGER: OnceCell<Mutex<POGRLogger>> = OnceCell::new();

pub fn init_logger(auth_config: LogConfig, api_url: Option<String>, logger_config: LoggerConfig, filter: LevelFilter) {
    //let _logger = LOGGER.get_or_init(|| Mutex::new(POGRLogger::new(config)));
    let _logger = POGRLogger::new(
        Client::new(),
        api_url, 
        auth_config,
        logger_config,
    );
    // Since set_logger requires a &'static dyn Log, we use a static function pointer to a function that
    // dereferences the logger from the LOGGER static. This requires implementing a static method that
    // can act as the Log implementation for the global logger.
    static LOG_FN: &(dyn Log + Sync + Send) = &LoggerFn;

    set_logger(LOG_FN).expect("Failed to set logger");
    set_max_level(filter);
}

struct LoggerFn;

impl Log for LoggerFn {
    fn enabled(&self, metadata: &Metadata) -> bool {
        LOGGER.get().unwrap().lock().unwrap().enabled(metadata)
    }

    fn log(&self, record: &Record) {
        LOGGER.get().unwrap().lock().unwrap().log(record)
    }

    fn flush(&self) {
        LOGGER.get().unwrap().lock().unwrap().flush()
    }
}