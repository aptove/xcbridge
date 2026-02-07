// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Configuration module for xcbridge

use clap::Parser;
use std::path::PathBuf;

/// Xcode bridge service for containerized iOS development
#[derive(Parser, Debug, Clone)]
#[command(name = "xcbridge")]
#[command(author = "Aptove")]
#[command(version)]
#[command(about = "Xcode bridge service for containerized iOS development", long_about = None)]
pub struct Config {
    /// Port to listen on
    #[arg(short, long, default_value = "9090", env = "XCBRIDGE_PORT")]
    pub port: u16,

    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1", env = "XCBRIDGE_HOST")]
    pub host: String,

    /// API key for authentication (optional)
    #[arg(long, env = "XCBRIDGE_API_KEY")]
    pub api_key: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info", env = "XCBRIDGE_LOG_LEVEL")]
    pub log_level: String,

    /// Allowed paths for build operations (security restriction)
    #[arg(long, env = "XCBRIDGE_ALLOWED_PATHS", value_delimiter = ',')]
    pub allowed_paths: Option<Vec<PathBuf>>,
}

impl Config {
    /// Parse configuration from CLI arguments
    pub fn parse_args() -> Self {
        Config::parse()
    }

    /// Check if a path is allowed for build operations
    pub fn is_path_allowed(&self, path: &PathBuf) -> bool {
        match &self.allowed_paths {
            Some(allowed) => {
                let canonical = path.canonicalize().ok();
                allowed.iter().any(|allowed_path| {
                    if let (Some(canonical), Ok(allowed_canonical)) =
                        (&canonical, allowed_path.canonicalize())
                    {
                        canonical.starts_with(&allowed_canonical)
                    } else {
                        false
                    }
                })
            }
            None => true, // No restrictions if not configured
        }
    }

    /// Get the socket address to bind to
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_addr() {
        let config = Config {
            port: 9090,
            host: "127.0.0.1".to_string(),
            api_key: None,
            log_level: "info".to_string(),
            allowed_paths: None,
        };
        assert_eq!(config.socket_addr(), "127.0.0.1:9090");
    }
}
