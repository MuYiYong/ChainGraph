//! 数据导入模块
//!
//! 支持从 CSV、JSON 批量导入区块链数据

use crate::error::{Error, Result};
use crate::graph::{Graph, VertexId};
use crate::types::{PropertyValue, TokenAmount, TxHash, VertexLabel};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 导入统计
#[derive(Debug, Default, Clone)]
pub struct ImportStats {
    pub vertices_imported: usize,
    pub edges_imported: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

/// 批量导入器
pub struct BatchImporter {
    graph: Arc<Graph>,
    batch_size: usize,
}

impl BatchImporter {
    /// 创建导入器
    pub fn new(graph: Arc<Graph>) -> Self {
        Self {
            graph,
            batch_size: 10000,
        }
    }

    /// 设置批次大小
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// 从 CSV 导入转账记录
    pub fn import_transfers_csv<P: AsRef<Path>>(&self, path: P) -> Result<ImportStats> {
        let start = std::time::Instant::now();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut stats = ImportStats::default();
        let mut lines = Vec::new();

        for line in reader.lines().skip(1) {
            // 跳过表头
            if let Ok(line) = line {
                lines.push(line);
            }
        }

        // 批量处理
        for chunk in lines.chunks(self.batch_size) {
            for line in chunk {
                match self.parse_and_import_transfer(line) {
                    Ok(_) => {
                        stats.vertices_imported += 2; // from + to
                        stats.edges_imported += 1;
                    }
                    Err(_) => stats.errors += 1,
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// 解析并导入单条转账
    fn parse_and_import_transfer(&self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 {
            return Err(Error::ImportError("CSV 格式错误".to_string()));
        }

        // 地址按字符串处理，不再解析为 native Address
        let from_addr = parts[0].trim().to_string();
        let to_addr = parts[1].trim().to_string();
        let amount = parts[2]
            .trim()
            .parse::<u64>()
            .map(TokenAmount::from_u64)
            .unwrap_or_else(|_| TokenAmount::from_u64(0));
        let block_number = parts[3].trim().parse::<u64>().unwrap_or(0);

        let from_id = self.graph.add_account(from_addr)?;
        let to_id = self.graph.add_account(to_addr)?;
        self.graph
            .add_transfer(from_id, to_id, amount, block_number)?;

        Ok(())
    }

    /// 从 JSON Lines 导入
    pub fn import_jsonl<P: AsRef<Path>>(&self, path: P) -> Result<ImportStats> {
        let start = std::time::Instant::now();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut stats = ImportStats::default();

        for line in reader.lines() {
            if let Ok(line) = line {
                match self.parse_and_import_json(&line) {
                    Ok((v, e)) => {
                        stats.vertices_imported += v;
                        stats.edges_imported += e;
                    }
                    Err(_) => stats.errors += 1,
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// 解析并导入 JSON 记录
    fn parse_and_import_json(&self, line: &str) -> Result<(usize, usize)> {
        let record: TransferRecord = serde_json::from_str(line)
            .map_err(|e| Error::ImportError(format!("JSON 解析错误: {}", e)))?;

        // JSON records contain address strings
        let from_addr = record.from.clone();
        let to_addr = record.to.clone();
        let amount = TokenAmount::from_u64(record.value.parse().unwrap_or(0));

        let from_id = self.graph.add_account(from_addr)?;
        let to_id = self.graph.add_account(to_addr)?;
        self.graph
            .add_transfer(from_id, to_id, amount, record.block_number)?;

        Ok((2, 1))
    }

    /// 并行导入（适合大文件）
    pub fn import_transfers_csv_parallel<P: AsRef<Path>>(&self, path: P) -> Result<ImportStats> {
        let start = std::time::Instant::now();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let lines: Vec<String> = reader.lines().skip(1).filter_map(|l| l.ok()).collect();

        let vertices_count = AtomicUsize::new(0);
        let edges_count = AtomicUsize::new(0);
        let errors_count = AtomicUsize::new(0);

        // 并行处理
        lines
            .par_iter()
            .for_each(|line| match self.parse_and_import_transfer(line) {
                Ok(_) => {
                    vertices_count.fetch_add(2, Ordering::Relaxed);
                    edges_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    errors_count.fetch_add(1, Ordering::Relaxed);
                }
            });

        Ok(ImportStats {
            vertices_imported: vertices_count.load(Ordering::Relaxed),
            edges_imported: edges_count.load(Ordering::Relaxed),
            errors: errors_count.load(Ordering::Relaxed),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// 导入交易记录
    pub fn import_transactions<P: AsRef<Path>>(&self, path: P) -> Result<ImportStats> {
        let start = std::time::Instant::now();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut stats = ImportStats::default();

        for line in reader.lines().skip(1) {
            if let Ok(line) = line {
                match self.parse_and_import_transaction(&line) {
                    Ok(_) => {
                        stats.vertices_imported += 1;
                    }
                    Err(_) => stats.errors += 1,
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    fn parse_and_import_transaction(&self, line: &str) -> Result<VertexId> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            return Err(Error::ImportError("CSV 格式错误".to_string()));
        }

        let tx_hash = TxHash::from_hex(parts[0].trim())?;
        let block_number = parts[1].trim().parse::<u64>().unwrap_or(0);

        let id = self.graph.add_vertex(VertexLabel::Transaction)?;

        if let Some(mut vertex) = self.graph.get_vertex(id) {
            vertex.set_property("tx_hash".to_string(), PropertyValue::TxHash(tx_hash));
            vertex.set_property(
                "block_number".to_string(),
                PropertyValue::Integer(block_number as i64),
            );
            self.graph.update_vertex(vertex)?;
        }

        Ok(id)
    }
}

/// 转账记录（JSON 格式）
#[derive(Debug, Serialize, Deserialize)]
struct TransferRecord {
    from: String,
    to: String,
    value: String,
    block_number: u64,
    #[serde(default)]
    tx_hash: Option<String>,
    #[serde(default)]
    token_address: Option<String>,
}

/// 从 Etherscan 风格的 CSV 导入
pub fn import_etherscan_csv<P: AsRef<Path>>(graph: Arc<Graph>, path: P) -> Result<ImportStats> {
    let importer = BatchImporter::new(graph);
    importer.import_transfers_csv(path)
}

/// 从 JSON Lines 导入
pub fn import_jsonl<P: AsRef<Path>>(graph: Arc<Graph>, path: P) -> Result<ImportStats> {
    let importer = BatchImporter::new(graph);
    importer.import_jsonl(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_import_csv() {
        let graph = Graph::in_memory().unwrap();
        let importer = BatchImporter::new(graph.clone());

        // 创建测试 CSV
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "from,to,value,block_number").unwrap();
        writeln!(
            file,
            "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0,0x8ba1f109551bD432803012645Ac136ddd64DBA72,1000,12345678"
        )
        .unwrap();

        let stats = importer.import_transfers_csv(file.path()).unwrap();
        assert_eq!(stats.vertices_imported, 2);
        assert_eq!(stats.edges_imported, 1);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_import_jsonl() {
        let graph = Graph::in_memory().unwrap();
        let importer = BatchImporter::new(graph.clone());

        // 创建测试 JSONL
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"from":"0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0","to":"0x8ba1f109551bD432803012645Ac136ddd64DBA72","value":"1000","block_number":12345678}}"#
        )
        .unwrap();

        let stats = importer.import_jsonl(file.path()).unwrap();
        assert_eq!(stats.vertices_imported, 2);
        assert_eq!(stats.edges_imported, 1);
    }
}
