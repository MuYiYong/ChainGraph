//! Graph catalog for multi-graph management
//!
//! Responsible for loading, creating, dropping and switching graphs on disk.

use crate::error::{Error, Result};
use crate::graph::Graph;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_GRAPH_NAME: &str = "default";
const CATALOG_FILE: &str = "catalog.json";

#[derive(Debug, Serialize, Deserialize, Default)]
struct CatalogMeta {
    current_graph: String,
    graphs: Vec<String>,
}

/// GraphCatalog maintains a registry of graph instances under a base data directory.
pub struct GraphCatalog {
    base_dir: PathBuf,
    buffer_pool_size: Option<usize>,
    current_graph: RwLock<String>,
    graphs: RwLock<HashMap<String, Arc<Graph>>>,
}

impl GraphCatalog {
    /// Open catalog at base_dir. If no catalog exists, create the default graph.
    pub fn open<P: AsRef<Path>>(base_dir: P, buffer_pool_size: Option<usize>) -> Result<Arc<Self>> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)
            .map_err(|e| Error::StorageError(format!("无法创建数据目录 {:?}: {}", base_dir, e)))?;

        let mut catalog = Self {
            base_dir: base_dir.clone(),
            buffer_pool_size,
            current_graph: RwLock::new(String::new()),
            graphs: RwLock::new(HashMap::new()),
        };

        // Load meta if exists; otherwise bootstrap default graph
        if let Some(meta) = catalog.load_meta()? {
            for name in &meta.graphs {
                let g = catalog.open_graph_dir(name)?;
                catalog.graphs.write().insert(name.clone(), g);
            }
            if meta.graphs.is_empty() {
                catalog.bootstrap_default()?;
            } else {
                *catalog.current_graph.write() = meta.current_graph;
            }
        } else {
            catalog.bootstrap_default()?;
        }

        Ok(Arc::new(catalog))
    }

    fn bootstrap_default(&mut self) -> Result<()> {
        let g = self.open_graph_dir(DEFAULT_GRAPH_NAME)?;
        self.graphs
            .write()
            .insert(DEFAULT_GRAPH_NAME.to_string(), g);
        *self.current_graph.write() = DEFAULT_GRAPH_NAME.to_string();
        self.save_meta()
    }

    fn open_graph_dir(&self, name: &str) -> Result<Arc<Graph>> {
        let dir = self.base_dir.join(name);
        Graph::open(dir, self.buffer_pool_size)
    }

    fn meta_path(&self) -> PathBuf {
        self.base_dir.join(CATALOG_FILE)
    }

    fn load_meta(&self) -> Result<Option<CatalogMeta>> {
        let path = self.meta_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read(&path)
            .map_err(|e| Error::StorageError(format!("读取 catalog 失败: {}", e)))?;
        let meta: CatalogMeta = serde_json::from_slice(&data)
            .map_err(|e| Error::StorageError(format!("解析 catalog 失败: {}", e)))?;
        Ok(Some(meta))
    }

    fn save_meta(&self) -> Result<()> {
        let graphs: Vec<String> = self.graphs.read().keys().cloned().collect();
        let meta = CatalogMeta {
            current_graph: self.current_graph.read().clone(),
            graphs,
        };
        let data = serde_json::to_vec_pretty(&meta)
            .map_err(|e| Error::StorageError(format!("序列化 catalog 失败: {}", e)))?;
        fs::write(self.meta_path(), data)
            .map_err(|e| Error::StorageError(format!("写入 catalog 失败: {}", e)))?
            ;
        Ok(())
    }

    /// Create a new graph and register it. Fails if name exists.
    pub fn create_graph(&self, name: &str) -> Result<Arc<Graph>> {
        if self.graphs.read().contains_key(name) {
            return Err(Error::QueryError(format!("Graph '{}' already exists", name)));
        }
        let dir = self.base_dir.join(name);
        fs::create_dir_all(&dir)
            .map_err(|e| Error::StorageError(format!("创建图目录失败: {}", e)))?;
        let graph = Graph::open(dir, self.buffer_pool_size)?;
        self.graphs.write().insert(name.to_string(), graph.clone());
        if self.current_graph.read().is_empty() {
            *self.current_graph.write() = name.to_string();
        }
        self.save_meta()?;
        Ok(graph)
    }

    /// Drop a graph and remove its directory.
    pub fn drop_graph(&self, name: &str) -> Result<()> {
        let mut graphs = self.graphs.write();
        if graphs.remove(name).is_none() {
            return Err(Error::QueryError(format!("Graph '{}' not found", name)));
        }
        let dir = self.base_dir.join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|e| Error::StorageError(format!("删除图目录失败: {}", e)))?;
        }
        // Adjust current graph if needed
        if *self.current_graph.read() == name {
            if let Some(next) = graphs.keys().next().cloned() {
                *self.current_graph.write() = next;
            } else {
                *self.current_graph.write() = String::new();
            }
        }
        self.save_meta()
    }

    /// Switch current graph.
    pub fn use_graph(&self, name: &str) -> Result<Arc<Graph>> {
        if let Some(g) = self.graphs.read().get(name) {
            *self.current_graph.write() = name.to_string();
            self.save_meta()?;
            return Ok(g.clone());
        }
        // Try open lazy if directory exists
        let dir = self.base_dir.join(name);
        if dir.exists() {
            let g = Graph::open(dir, self.buffer_pool_size)?;
            self.graphs.write().insert(name.to_string(), g.clone());
            *self.current_graph.write() = name.to_string();
            self.save_meta()?;
            return Ok(g);
        }
        Err(Error::QueryError(format!("Graph '{}' not found", name)))
    }

    /// Get current graph instance.
    pub fn current_graph(&self) -> Arc<Graph> {
        let name = self.current_graph.read().clone();
        if let Some(g) = self.graphs.read().get(&name) {
            return g.clone();
        }
        // Fallback: bootstrap default
        self.open_graph_dir(DEFAULT_GRAPH_NAME)
            .map(|g| {
                self.graphs
                    .write()
                    .insert(DEFAULT_GRAPH_NAME.to_string(), g.clone());
                *self.current_graph.write() = DEFAULT_GRAPH_NAME.to_string();
                let _ = self.save_meta();
                g
            })
            .expect("Graph catalog missing default graph")
    }

    pub fn current_graph_name(&self) -> String {
        self.current_graph.read().clone()
    }

    /// List graph names.
    pub fn list_graphs(&self) -> Vec<String> {
        self.graphs.read().keys().cloned().collect()
    }

    /// Get a specific graph by name without switching context.
    pub fn get_graph(&self, name: &str) -> Option<Arc<Graph>> {
        self.graphs.read().get(name).cloned()
    }

    /// Ensure a graph is loaded (without changing current) and return it.
    pub fn ensure_graph(&self, name: &str) -> Result<Arc<Graph>> {
        if let Some(g) = self.get_graph(name) {
            return Ok(g);
        }
        let dir = self.base_dir.join(name);
        if dir.exists() {
            let g = Graph::open(dir, self.buffer_pool_size)?;
            self.graphs.write().insert(name.to_string(), g.clone());
            self.save_meta()?;
            return Ok(g);
        }
        Err(Error::QueryError(format!("Graph '{}' not found", name)))
    }
}
