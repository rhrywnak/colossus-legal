use neo4rs::Graph;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub graph: Graph,
    pub config: AppConfig,
}
