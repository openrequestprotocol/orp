//! ORP federated home server.

pub mod auth;
pub mod config;
pub mod db;
pub mod deliver;
pub mod discovery;
pub mod keys;
pub mod routes;
pub mod smtp;
pub mod state;
pub mod webhook;

pub use config::ServerConfig;
pub use state::AppState;
