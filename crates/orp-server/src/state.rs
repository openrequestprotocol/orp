use std::collections::HashMap;
use std::sync::Arc;

use orp_core::{
    BudgetTracker, DiscoveryDocument, KeyPair, PublicKeyBundle, ReputationStore,
};
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::config::ServerConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: ServerConfig,
    pub keypair: Arc<KeyPair>,
    pub discovery: Arc<DiscoveryDocument>,
    pub budgets: Arc<RwLock<HashMap<String, BudgetTracker>>>,
    pub reputation: Arc<RwLock<HashMap<String, ReputationStore>>>,
}

impl AppState {
    pub fn new(pool: PgPool, config: ServerConfig) -> Self {
        let keypair = if let Some(seed) = config.signing_seed {
            KeyPair::from_seed("server-key-1", &seed)
        } else {
            KeyPair::generate("server-key-1")
        };
        let discovery = DiscoveryDocument::new(
            &config.public_endpoint,
            vec![keypair.public_bundle()],
        );
        Self {
            pool,
            config,
            keypair: Arc::new(keypair),
            discovery: Arc::new(discovery),
            budgets: Arc::new(RwLock::new(HashMap::new())),
            reputation: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn server_public_keys(&self) -> Vec<PublicKeyBundle> {
        vec![self.keypair.public_bundle()]
    }
}
