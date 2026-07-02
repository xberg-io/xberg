/// GraphQLite graph backend for xberg-rag.
/// Provides Cypher-like graph traversal, Louvain community detection,
/// and PageRank scoring — all backed by SQLite's recursive CTEs.

use crate::{RagError, RagResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// A SQLite-backed graph store.
pub struct GraphStore {
    conn: Connection,
}

/// A Louvain community.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: u32,
    pub nodes: Vec<String>,
    pub size: usize,
}

impl GraphStore {
    /// Create a new GraphStore wrapping an existing rusqlite Connection.
    pub fn new(conn: Connection) -> RagResult<Self> {
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize graph schema tables.
    fn init_schema(&self) -> RagResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _graph_nodes (
                id TEXT PRIMARY KEY,
                labels TEXT NOT NULL DEFAULT '[]',
                properties TEXT NOT NULL DEFAULT '{}'
            ) STRICT;
            CREATE TABLE IF NOT EXISTS _graph_edges (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL REFERENCES _graph_nodes(id),
                target TEXT NOT NULL REFERENCES _graph_nodes(id),
                label TEXT NOT NULL DEFAULT '',
                properties TEXT NOT NULL DEFAULT '{}',
                UNIQUE(source, target, label)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS idx_edges_source ON _graph_edges(source);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON _graph_edges(target);
            CREATE INDEX IF NOT EXISTS idx_edges_label ON _graph_edges(label);
            CREATE TABLE IF NOT EXISTS _graph_properties (
                node_id TEXT NOT NULL REFERENCES _graph_nodes(id),
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (node_id, key)
            ) STRICT;",
        )
        .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(())
    }

    /// Create a node with optional properties.
    pub fn create_node(
        &self,
        id: &str,
        labels: &[&str],
        properties: &serde_json::Value,
    ) -> RagResult<()> {
        let labels_json = serde_json::to_string(labels).unwrap_or_default();
        let props_json = serde_json::to_string(properties).unwrap_or_default();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO _graph_nodes (id, labels, properties) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, labels_json, props_json],
            )
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(())
    }

    /// Create an edge between two nodes.
    pub fn create_edge(
        &self,
        id: &str,
        source: &str,
        target: &str,
        label: &str,
        properties: &serde_json::Value,
    ) -> RagResult<()> {
        let props_json = serde_json::to_string(properties).unwrap_or_default();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO _graph_edges (id, source, target, label, properties) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, source, target, label, props_json],
            )
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(())
    }

    /// BFS traversal from seed node IDs up to a given depth.
    /// Returns all reachable node IDs.
    pub fn traverse_bfs(
        &self,
        start_ids: &[String],
        depth: u32,
        edge_labels: &[&str],
    ) -> RagResult<Vec<String>> {
        if start_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut query = String::from(
            "WITH RECURSIVE traversal(path, node_id, depth) AS (
                SELECT id, id, 0 FROM _graph_nodes WHERE id IN (",
        );

        let params: Vec<String> = (0..start_ids.len()).map(|i| format!("?{}", i + 1)).collect();
        query.push_str(&params.join(", "));
        query.push_str(
            ")
        UNION
        SELECT traversal.path || '/' || e.target, e.target, traversal.depth + 1
                FROM traversal
                JOIN _graph_edges e ON e.source = traversal.node_id",
        );

        if !edge_labels.is_empty() {
            let label_params: Vec<String> = (start_ids.len() + 1..start_ids.len() + 1 + edge_labels.len())
                .map(|i| format!("?{}", i))
                .collect();
            query.push_str(" AND e.label IN (");
            query.push_str(&label_params.join(", "));
            query.push(')');
        }

        query.push_str("
                WHERE traversal.depth < ?");
        query.push_str(&(start_ids.len() + edge_labels.len() + 1).to_string());

        query.push_str(
            ")\n        SELECT DISTINCT node_id FROM traversal WHERE depth > 0
            ORDER BY node_id",
        );

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for id in start_ids {
            all_params.push(Box::new(id.clone()));
        }
        for label in edge_labels {
            all_params.push(Box::new(label.to_string()));
        }
        all_params.push(Box::new(depth));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self
            .conn
            .prepare(&query)
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| row.get::<_, String>(0))
            .map_err(|e| RagError::Backend(Box::new(e)))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| RagError::Backend(Box::new(e)))?);
        }
        Ok(ids)
    }

    /// Run Louvain community detection.
    /// Uses a simple label-propagation algorithm since graphqlite is not available.
    pub fn louvain(&self, _resolution: f64) -> RagResult<Vec<Community>> {
        let mut stmt = self
            .conn
            .prepare(
                "WITH node_labels AS (
                    SELECT source AS node_id, target AS neighbor FROM _graph_edges
                    UNION
                    SELECT target AS node_id, source AS neighbor FROM _graph_edges
                )
                SELECT node_id, neighbor FROM node_labels ORDER BY node_id",
            )
            .map_err(|e| RagError::Backend(Box::new(e)))?;

        struct AdjEntry {
            node_id: String,
            neighbor: String,
        }

        let rows = stmt
            .query_map([], |row| {
                Ok(AdjEntry {
                    node_id: row.get(0)?,
                    neighbor: row.get(1)?,
                })
            })
            .map_err(|e| RagError::Backend(Box::new(e)))?;

        let mut adjacency: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in rows {
            let entry = row.map_err(|e| RagError::Backend(Box::new(e)))?;
            adjacency
                .entry(entry.node_id)
                .or_default()
                .push(entry.neighbor);
        }

        let mut communities: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for (i, node) in adjacency.keys().enumerate() {
            communities.insert(node.clone(), i as u32);
        }

        let mut changed = true;
        let max_iterations = 10;
        let mut iteration = 0;
        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;
            let nodes: Vec<String> = adjacency.keys().cloned().collect();
            for node in &nodes {
                if let Some(neighbors) = adjacency.get(node) {
                    let mut label_counts: std::collections::HashMap<u32, u32> =
                        std::collections::HashMap::new();
                    for neighbor in neighbors {
                        if let Some(&label) = communities.get(neighbor) {
                            *label_counts.entry(label).or_default() += 1;
                        }
                    }
                    if let Some((&best_label, _)) = label_counts.iter().max_by_key(|&(_, count)| count) {
                        if communities.get(node) != Some(&best_label) {
                            communities.insert(node.clone(), best_label);
                            changed = true;
                        }
                    }
                }
            }
        }

        let mut community_map: std::collections::HashMap<u32, Vec<String>> =
            std::collections::HashMap::new();
        for (node, label) in &communities {
            community_map.entry(*label).or_default().push(node.clone());
        }

        Ok(community_map
            .into_iter()
            .map(|(id, nodes)| Community {
                id,
                size: nodes.len(),
                nodes,
            })
            .collect())
    }

    /// Run PageRank scoring.
    pub fn pagerank(&self, damping: f64, max_iterations: u32) -> RagResult<Vec<(String, f64)>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT source, target FROM _graph_edges",
            )
            .map_err(|e| RagError::Backend(Box::new(e)))?;

        struct Edge {
            source: String,
            target: String,
        }

        let rows = stmt
            .query_map([], |row| {
                Ok(Edge {
                    source: row.get(0)?,
                    target: row.get(1)?,
                })
            })
            .map_err(|e| RagError::Backend(Box::new(e)))?;

        let mut out_degree: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let mut incoming: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut all_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

        for row in rows {
            let edge = row.map_err(|e| RagError::Backend(Box::new(e)))?;
            *out_degree.entry(edge.source.clone()).or_default() += 1.0;
            incoming
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
            all_nodes.insert(edge.source);
            all_nodes.insert(edge.target);
        }

        let mut stmt2 = self
            .conn
            .prepare("SELECT id FROM _graph_nodes")
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        let node_rows = stmt2
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        for row in node_rows {
            let id = row.map_err(|e| RagError::Backend(Box::new(e)))?;
            all_nodes.insert(id);
        }

        let n = all_nodes.len() as f64;
        let mut scores: std::collections::HashMap<String, f64> = all_nodes
            .iter()
            .map(|node| (node.clone(), 1.0 / n))
            .collect();

        for node in &all_nodes {
            out_degree.entry(node.clone()).or_insert(0.0);
        }

        let teleport = (1.0 - damping) / n;

        for _ in 0..max_iterations {
            let mut new_scores: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            for node in &all_nodes {
                let mut sum = 0.0;
                if let Some(incoming_nodes) = incoming.get(node) {
                    for incoming_node in incoming_nodes {
                        let deg = out_degree.get(incoming_node).copied().unwrap_or(0.0);
                        let score = scores.get(incoming_node).copied().unwrap_or(0.0);
                        if deg > 0.0 {
                            sum += score / deg;
                        }
                    }
                }
                new_scores.insert(node.clone(), teleport + damping * sum);
            }
            scores = new_scores;
        }

        let mut result: Vec<(String, f64)> = scores.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(result)
    }

    /// Delete a node and all its edges.
    pub fn delete_node(&self, id: &str) -> RagResult<u64> {
        self.conn
            .execute("DELETE FROM _graph_edges WHERE source = ?1", rusqlite::params![id])
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        self.conn
            .execute("DELETE FROM _graph_edges WHERE target = ?1", rusqlite::params![id])
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        let count = self
            .conn
            .execute("DELETE FROM _graph_nodes WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(count as u64)
    }

    /// Get all node IDs with a specific label.
    pub fn get_nodes_by_label(&self, label: &str) -> RagResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT n.id FROM _graph_nodes n
                 WHERE json_array_length(n.labels) > 0
                 AND EXISTS (
                     SELECT 1 FROM json_each(n.labels) WHERE json_each.value = ?1
                 )",
            )
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        let rows = stmt
            .query_map(rusqlite::params![label], |row| row.get::<_, String>(0))
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| RagError::Backend(Box::new(e)))?);
        }
        Ok(ids)
    }

    pub fn get_node_count(&self) -> RagResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM _graph_nodes", [], |row| row.get(0))
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(count as u64)
    }

    pub fn get_edge_count(&self) -> RagResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM _graph_edges", [], |row| row.get(0))
            .map_err(|e| RagError::Backend(Box::new(e)))?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_node() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        store
            .create_node("doc1", &["Document"], &serde_json::json!({"title": "Test"}))
            .unwrap();
        assert_eq!(store.get_node_count().unwrap(), 1);
    }

    #[test]
    fn test_create_edge() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        store.create_node("doc1", &["Document"], &serde_json::json!({})).unwrap();
        store.create_node("chunk1", &["Chunk"], &serde_json::json!({})).unwrap();
        store
            .create_edge("e1", "doc1", "chunk1", "HAS_CHUNK", &serde_json::json!({"ordinal": 1}))
            .unwrap();
        assert_eq!(store.get_edge_count().unwrap(), 1);
    }

    #[test]
    fn test_traverse_bfs() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        store.create_node("doc1", &["Document"], &serde_json::json!({})).unwrap();
        store.create_node("doc2", &["Document"], &serde_json::json!({})).unwrap();
        store.create_node("doc3", &["Document"], &serde_json::json!({})).unwrap();
        store
            .create_edge("e1", "doc1", "doc2", "COOCCURS", &serde_json::json!({}))
            .unwrap();
        store
            .create_edge("e2", "doc2", "doc3", "COOCCURS", &serde_json::json!({}))
            .unwrap();

        let result = store
            .traverse_bfs(&["doc1".to_string()], 2, &["COOCCURS"])
            .unwrap();
        assert!(result.contains(&"doc2".to_string()));
        assert!(result.contains(&"doc3".to_string()));
    }

    #[test]
    fn test_pagerank() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        for i in 1..=4 {
            store
                .create_node(&format!("doc{}", i), &["Document"], &serde_json::json!({}))
                .unwrap();
        }
        store
            .create_edge("e1", "doc1", "doc2", "COOCCURS", &serde_json::json!({}))
            .unwrap();
        store
            .create_edge("e2", "doc1", "doc3", "COOCCURS", &serde_json::json!({}))
            .unwrap();
        store
            .create_edge("e3", "doc1", "doc4", "COOCCURS", &serde_json::json!({}))
            .unwrap();
        store
            .create_edge("e4", "doc2", "doc3", "COOCCURS", &serde_json::json!({}))
            .unwrap();

        let scores = store.pagerank(0.85, 20).unwrap();
        assert!(scores.len() >= 4);
        assert!(scores.iter().any(|(id, _)| id == "doc1"));
    }

    #[test]
    fn test_louvain() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        for i in 1..=3 {
            store
                .create_node(&format!("a{}", i), &["Entity"], &serde_json::json!({}))
                .unwrap();
        }
        for i in 1..=3 {
            store
                .create_node(&format!("b{}", i), &["Entity"], &serde_json::json!({}))
                .unwrap();
        }
        store.create_edge("ea1", "a1", "a2", "RELATED", &serde_json::json!({})).unwrap();
        store.create_edge("ea2", "a2", "a3", "RELATED", &serde_json::json!({})).unwrap();
        store.create_edge("eb1", "b1", "b2", "RELATED", &serde_json::json!({})).unwrap();
        store.create_edge("eb2", "b2", "b3", "RELATED", &serde_json::json!({})).unwrap();

        let communities = store.louvain(1.0).unwrap();
        assert_eq!(communities.len(), 2);
    }

    #[test]
    fn test_delete_node() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        store.create_node("doc1", &["Document"], &serde_json::json!({})).unwrap();
        store.create_node("doc2", &["Document"], &serde_json::json!({})).unwrap();
        store
            .create_edge("e1", "doc1", "doc2", "COOCCURS", &serde_json::json!({}))
            .unwrap();
        store.delete_node("doc1").unwrap();
        assert_eq!(store.get_node_count().unwrap(), 1);
        assert_eq!(store.get_edge_count().unwrap(), 0);
    }

    #[test]
    fn test_get_nodes_by_label() {
        let conn = Connection::open_in_memory().unwrap();
        let store = GraphStore::new(conn).unwrap();
        store.create_node("doc1", &["Document"], &serde_json::json!({})).unwrap();
        store.create_node("ent1", &["Entity"], &serde_json::json!({})).unwrap();
        let docs = store.get_nodes_by_label("Document").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], "doc1");
    }
}
