pub mod cli;
pub mod config;
pub mod datadog;
pub mod error;
pub mod handlers;
pub mod utils;

mod diagnostics;

pub use config::Config;
pub use datadog::DatadogClient;
pub use error::{DatadogError, Result};

pub(crate) mod output;

mod auth;

pub mod catalog;
