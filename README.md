# POGR Log SDK for Rust

The POGR Log SDK is the official Rust logging library developed by Randolph William Aarseth II for POGR, designed to offer structured logging with remote log aggregation capabilities. It leverages the power of `log` and `reqwest` to provide a flexible, easy-to-use interface for sending logs to the POGR logging platform, supporting both synchronous and asynchronous logging.

## Features

- **Structured Logging**: Easily log structured data as JSON, making your logs more searchable and analyzable.
- **Remote Log Aggregation**: Seamlessly send your logs to the POGR logging platform for centralized log management.
- **Flexible Authentication**: Supports client-based and API key-based authentication methods.
- **Asynchronous Support**: Utilizes async/await for non-blocking log transmissions.
- **Environment-Based Configuration**: Configure logging details through environment variables for easy setup and changes.

## Getting Started

### Prerequisites

Ensure you have the latest Rust compiler and Cargo installed. This SDK is compatible with Rust 2018 edition and later.

### Installation

Add the POGR Log SDK to your `Cargo.toml`:

```toml
[dependencies]
pogr_log_sdk = "0.1.0"
```

### Basic Configuration

To use the SDK, you need to configure the logger with your POGR credentials and desired log level:

```rust
use pogr_log_sdk::{init_logger, LogConfig, LevelFilter};

fn main() {
    let config = LogConfig::AccessKeys {
        access_key: "your_access_key".to_string(),
        secret_key: "your_secret_key".to_string(),
        logger_config: LoggerConfig {
            service: "your_service_name".to_string(),
            environment: "your_environment".to_string(),
            default_type: Some("default_log_type".to_string()),
        },
    };

    init_logger(config, LevelFilter::Info);
}
```

## Usage

Logging with the POGR Log SDK is simple. Here's how you can log different types of information:

```rust
use log::{info, warn, error};

fn perform_action() {
    info!("This is an informational message");
    warn!("This is a warning message");
    error!("This is an error message");
}
```

For structured logging:

```rust
use pogr_log_sdk::structured_log;
use log::Level;

fn user_login(user_id: u64) {
    structured_log!(Level::Info, "User logged in", "login", {"user_id": user_id}, {"env": "production"});
}
```

## Configuration

The POGR Log SDK is designed to be highly configurable to suit various logging needs and environments. This section covers the in-depth configuration options available to tailor the SDK to your specific requirements.

### Authentication Configuration

The SDK supports two primary authentication methods: **Client-Build** and **Access Keys**. Depending on your POGR platform setup, you may choose one over the other. Here's how to configure each method:

#### Client-Build Authentication

This method is ideal for applications with a client and build ID system. It requires specifying both `client_id` and `build_id` along with the logger configuration.

```rust
use pogr_log_sdk::{LogConfig, LoggerConfig};

let config = LogConfig::ClientBuild {
    client_id: "your_client_id".to_string(),
    build_id: "your_build_id".to_string(),
    logger_config: LoggerConfig {
        service: "your_service_name".to_string(),
        environment: "your_environment".to_string(),
        default_type: None, // Optional
    },
};
```

#### Access Keys Authentication

For systems that use API keys for authentication, this method requires an `access_key` and a `secret_key`.

```rust
use pogr_log_sdk::{LogConfig, LoggerConfig};

let config = LogConfig::AccessKeys {
    access_key: "your_access_key".to_string(),
    secret_key: "your_secret_key".to_string(),
    logger_config: LoggerConfig {
        service: "your_service_name".to_string(),
        environment: "your_environment".to_string(),
        default_type: Some("default_log_type".to_string()), // Optional
    },
};
```

### Logger Configuration

The `LoggerConfig` struct allows you to specify global settings for your logs, such as the service name, environment, and a default log type.

- **Service**: A string representing the name of your service. This helps in filtering logs coming from different services.
- **Environment**: The environment where your service is running, such as `production`, `development`, or `staging`. This aids in segregating logs from different stages of your deployment pipeline.
- **Default Type**: An optional default type for your logs, useful for categorizing logs when a specific type is not provided.

### Log Level Filtering

Control the verbosity of your logs with log level filtering. The SDK supports the standard log levels: `Error`, `Warn`, `Info`, `Debug`, and `Trace`. You can set the maximum log level to ensure that only logs of that level or higher are sent to the POGR platform.

```rust
use log::LevelFilter;
use pogr_log_sdk::init_logger;

init_logger(config, LevelFilter::Info); // Only logs of Info level or higher will be processed.
```

### Environmental Variables

The SDK can also be configured via environmental variables, allowing for dynamic adjustments without code changes. Here are some of the supported variables:

- **POGR_INTAKE_URL**: The URL for the log intake API. This is useful if you have multiple environments or custom endpoints.

### Custom Log Fields

In addition to the predefined fields, you can include custom data with each log message using the `structured_log` macro. This enables you to attach relevant contextual information to your logs, enhancing their usefulness for debugging and analysis.

```rust
structured_log!(Level::Info, "User action", "user_event", {"user_id": 42, "action": "login"}, {"platform": "web"});
```

### Asynchronous Logging

The SDK performs logging operations asynchronously, ensuring minimal impact on your application's performance. Logs are sent in the background, leveraging Rust's async/await features for efficient operation.

### Error Handling and Retries

While the SDK prioritizes a fire-and-forget approach for simplicity, it's designed to gracefully handle transmission errors. You can extend the SDK to implement custom error handling or retry mechanisms based on your needs.

### Conclusion

The POGR Log SDK offers a flexible and powerful logging solution for Rust applications, with extensive configuration options to meet the needs of any project. By leveraging these configurations, you can ensure that your logging strategy is optimized for both development and production environments, providing clear insights into your application's behavior and performance.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Randolph William Aarseth II, for his vision and dedication to creating a robust logging solution for Rust applications.
- The Rust community, for providing invaluable resources and support.