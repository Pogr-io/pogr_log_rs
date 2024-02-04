use pogr_log_rs::LogConfig;
use pogr_log_rs::LoggerConfig;
use pogr_log_rs::POGRLogger;
use reqwest::Client;
use log::Level;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;


#[cfg(test)]
mod tests {
    use super::*;

    static INIT: Lazy<Arc<Mutex<()>>> = Lazy::new(|| {
        Arc::new(Mutex::new(()))
    });

    #[tokio::test]
    async fn test_custom_log_sends_correct_request() {
        let _lock = INIT.lock().await;
        let expected_body = json!({
            "service": "test_service",
            "environment": "test_env",
            "severity": "info",
            "type": "test_log",
            "log": "This is a test log",
            "data": {"test": "data"},
            "tags": {"tag1": "value1"},
        }).to_string();
        // Request a new server from the pool
        let mut server = mockito::Server::new();

        // Use one of these addresses to configure your client
        let _host = server.host_with_port();
        let base_url = server.url();
        let full_url = format!("{}/v1/intake/logs", base_url.trim_end_matches('/')); // Ensure no double slashes
        println!("Mock server URL: {}", full_url); // Log the URL to the console

        let _m = server.mock("POST", "/v1/intake/logs")
            .match_header("content-type", "application/json")
            .match_body(expected_body.as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"success\":true}")
            .create_async().await;

            let logger = POGRLogger::new(
                Client::new(),
                Some(full_url),
                LogConfig::AccessKeys {
                    access_key: "test_access_key".to_string(),
                    secret_key: "test_secret_key".to_string(),
                    logger_config: LoggerConfig {
                        service: "test_service".to_string(),
                        environment: "test_env".to_string(),
                        default_type: None,
                    },
                },
                LoggerConfig {
                    service: "test_service".to_string(),
                    environment: "test_env".to_string(),
                    default_type: None,
                },
            );
            

        logger.custom_log(Level::Info, "This is a test log", "test_log", json!({"test": "data"}), json!({"tag1": "value1"})).await;

        _m.assert_async().await;
    }
}
