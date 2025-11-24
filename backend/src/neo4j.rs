use neo4rs::{Graph, Query};
use tracing::info;
use crate::config::AppConfig;

//
// Create the Graph connection from the environment config
//
pub async fn create_neo4j_graph(config: &AppConfig) -> Result<Graph, neo4rs::Error> {
    // Neo4rs::Graph::new takes (uri, username, password)
    let graph = Graph::new(
        config.neo4j_uri.clone(),
        config.neo4j_user.clone(),
        config.neo4j_password.clone(),
    )
    .await?;

    info!(
        "Connected to Neo4j at {} as {}",
        config.neo4j_uri, config.neo4j_user
    );

    Ok(graph)
}

//
// Ping Neo4j by running a trivial query
//
pub async fn check_neo4j(graph: &Graph) -> Result<(), neo4rs::Error> {
    let mut result = graph.execute(Query::new("RETURN 1".into())).await?;
    let _ = result.next().await; // ensure Neo4j responded
    Ok(())
}
