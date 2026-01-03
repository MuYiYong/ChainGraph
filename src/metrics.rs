//! 性能指标收集模块
//!
//! 提供系统运行时性能指标的收集和导出功能

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 系统全局指标
#[derive(Debug)]
pub struct Metrics {
    /// 查询统计
    query_stats: QueryStats,
    /// 缓冲池统计
    buffer_pool_stats: BufferPoolStats,
    /// 图操作统计
    graph_stats: GraphStats,
    /// 启动时间
    start_time: Instant,
}

/// 查询统计
#[derive(Debug)]
struct QueryStats {
    /// 总查询数
    total_queries: AtomicU64,
    /// 成功查询数
    success_queries: AtomicU64,
    /// 失败查询数
    failed_queries: AtomicU64,
    /// 查询总耗时（微秒）
    total_duration_us: AtomicU64,
    /// 慢查询数（>1s）
    slow_queries: AtomicU64,
}

/// 缓冲池统计
#[derive(Debug)]
struct BufferPoolStats {
    /// 页面命中数
    hits: AtomicU64,
    /// 页面未命中数
    misses: AtomicU64,
    /// 页面驱逐数
    evictions: AtomicU64,
    /// 脏页写回数
    dirty_writes: AtomicU64,
}

/// 图操作统计
#[derive(Debug)]
struct GraphStats {
    /// 顶点插入数
    vertices_inserted: AtomicU64,
    /// 边插入数
    edges_inserted: AtomicU64,
    /// 顶点查询数
    vertices_queried: AtomicU64,
    /// 边查询数
    edges_queried: AtomicU64,
}

/// 可导出的指标快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    // 查询指标
    pub total_queries: u64,
    pub success_queries: u64,
    pub failed_queries: u64,
    pub avg_query_duration_ms: f64,
    pub slow_queries: u64,
    pub qps: f64,
    
    // 缓冲池指标
    pub buffer_pool_hits: u64,
    pub buffer_pool_misses: u64,
    pub buffer_pool_hit_rate: f64,
    pub buffer_pool_evictions: u64,
    pub buffer_pool_dirty_writes: u64,
    
    // 图操作指标
    pub vertices_inserted: u64,
    pub edges_inserted: u64,
    pub vertices_queried: u64,
    pub edges_queried: u64,
    
    // 系统指标
    pub uptime_seconds: u64,
}

/// Prometheus 格式指标
#[derive(Debug, Clone)]
pub struct PrometheusMetrics {
    pub content: String,
}

impl Metrics {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            query_stats: QueryStats {
                total_queries: AtomicU64::new(0),
                success_queries: AtomicU64::new(0),
                failed_queries: AtomicU64::new(0),
                total_duration_us: AtomicU64::new(0),
                slow_queries: AtomicU64::new(0),
            },
            buffer_pool_stats: BufferPoolStats {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                dirty_writes: AtomicU64::new(0),
            },
            graph_stats: GraphStats {
                vertices_inserted: AtomicU64::new(0),
                edges_inserted: AtomicU64::new(0),
                vertices_queried: AtomicU64::new(0),
                edges_queried: AtomicU64::new(0),
            },
            start_time: Instant::now(),
        }
    }

    /// 记录查询开始
    pub fn record_query_start(&self) -> QueryTimer {
        self.query_stats.total_queries.fetch_add(1, Ordering::Relaxed);
        QueryTimer::new()
    }

    /// 记录查询完成
    pub fn record_query_complete(&self, timer: QueryTimer, success: bool) {
        let duration = timer.elapsed();
        
        if success {
            self.query_stats.success_queries.fetch_add(1, Ordering::Relaxed);
        } else {
            self.query_stats.failed_queries.fetch_add(1, Ordering::Relaxed);
        }
        
        self.query_stats
            .total_duration_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        // 慢查询：超过1秒
        if duration.as_secs() >= 1 {
            self.query_stats.slow_queries.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 记录缓冲池命中
    pub fn record_buffer_hit(&self) {
        self.buffer_pool_stats.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录缓冲池未命中
    pub fn record_buffer_miss(&self) {
        self.buffer_pool_stats.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录页面驱逐
    pub fn record_eviction(&self) {
        self.buffer_pool_stats.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录脏页写回
    pub fn record_dirty_write(&self) {
        self.buffer_pool_stats.dirty_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录顶点插入
    pub fn record_vertex_insert(&self) {
        self.graph_stats.vertices_inserted.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录边插入
    pub fn record_edge_insert(&self) {
        self.graph_stats.edges_inserted.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录顶点查询
    pub fn record_vertex_query(&self) {
        self.graph_stats.vertices_queried.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录边查询
    pub fn record_edge_query(&self) {
        self.graph_stats.edges_queried.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取指标快照
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total_queries = self.query_stats.total_queries.load(Ordering::Relaxed);
        let success_queries = self.query_stats.success_queries.load(Ordering::Relaxed);
        let failed_queries = self.query_stats.failed_queries.load(Ordering::Relaxed);
        let total_duration_us = self.query_stats.total_duration_us.load(Ordering::Relaxed);
        let slow_queries = self.query_stats.slow_queries.load(Ordering::Relaxed);
        
        let hits = self.buffer_pool_stats.hits.load(Ordering::Relaxed);
        let misses = self.buffer_pool_stats.misses.load(Ordering::Relaxed);
        let evictions = self.buffer_pool_stats.evictions.load(Ordering::Relaxed);
        let dirty_writes = self.buffer_pool_stats.dirty_writes.load(Ordering::Relaxed);
        
        let uptime = self.start_time.elapsed().as_secs();
        
        let avg_query_duration_ms = if total_queries > 0 {
            (total_duration_us as f64) / (total_queries as f64) / 1000.0
        } else {
            0.0
        };
        
        let hit_rate = if hits + misses > 0 {
            (hits as f64) / ((hits + misses) as f64)
        } else {
            0.0
        };
        
        let qps = if uptime > 0 {
            (total_queries as f64) / (uptime as f64)
        } else {
            0.0
        };
        
        MetricsSnapshot {
            total_queries,
            success_queries,
            failed_queries,
            avg_query_duration_ms,
            slow_queries,
            qps,
            buffer_pool_hits: hits,
            buffer_pool_misses: misses,
            buffer_pool_hit_rate: hit_rate,
            buffer_pool_evictions: evictions,
            buffer_pool_dirty_writes: dirty_writes,
            vertices_inserted: self.graph_stats.vertices_inserted.load(Ordering::Relaxed),
            edges_inserted: self.graph_stats.edges_inserted.load(Ordering::Relaxed),
            vertices_queried: self.graph_stats.vertices_queried.load(Ordering::Relaxed),
            edges_queried: self.graph_stats.edges_queried.load(Ordering::Relaxed),
            uptime_seconds: uptime,
        }
    }

    /// 导出为 Prometheus 格式
    pub fn to_prometheus(&self) -> PrometheusMetrics {
        let snapshot = self.snapshot();
        
        let mut content = String::new();
        
        // 查询指标
        content.push_str("# HELP chaingraph_queries_total Total number of queries\n");
        content.push_str("# TYPE chaingraph_queries_total counter\n");
        content.push_str(&format!("chaingraph_queries_total {}\n", snapshot.total_queries));
        
        content.push_str("# HELP chaingraph_queries_success_total Number of successful queries\n");
        content.push_str("# TYPE chaingraph_queries_success_total counter\n");
        content.push_str(&format!("chaingraph_queries_success_total {}\n", snapshot.success_queries));
        
        content.push_str("# HELP chaingraph_queries_failed_total Number of failed queries\n");
        content.push_str("# TYPE chaingraph_queries_failed_total counter\n");
        content.push_str(&format!("chaingraph_queries_failed_total {}\n", snapshot.failed_queries));
        
        content.push_str("# HELP chaingraph_query_duration_avg_ms Average query duration in milliseconds\n");
        content.push_str("# TYPE chaingraph_query_duration_avg_ms gauge\n");
        content.push_str(&format!("chaingraph_query_duration_avg_ms {:.2}\n", snapshot.avg_query_duration_ms));
        
        content.push_str("# HELP chaingraph_slow_queries_total Number of slow queries (>1s)\n");
        content.push_str("# TYPE chaingraph_slow_queries_total counter\n");
        content.push_str(&format!("chaingraph_slow_queries_total {}\n", snapshot.slow_queries));
        
        content.push_str("# HELP chaingraph_qps Queries per second\n");
        content.push_str("# TYPE chaingraph_qps gauge\n");
        content.push_str(&format!("chaingraph_qps {:.2}\n", snapshot.qps));
        
        // 缓冲池指标
        content.push_str("# HELP chaingraph_buffer_pool_hits_total Buffer pool cache hits\n");
        content.push_str("# TYPE chaingraph_buffer_pool_hits_total counter\n");
        content.push_str(&format!("chaingraph_buffer_pool_hits_total {}\n", snapshot.buffer_pool_hits));
        
        content.push_str("# HELP chaingraph_buffer_pool_misses_total Buffer pool cache misses\n");
        content.push_str("# TYPE chaingraph_buffer_pool_misses_total counter\n");
        content.push_str(&format!("chaingraph_buffer_pool_misses_total {}\n", snapshot.buffer_pool_misses));
        
        content.push_str("# HELP chaingraph_buffer_pool_hit_rate Buffer pool hit rate (0-1)\n");
        content.push_str("# TYPE chaingraph_buffer_pool_hit_rate gauge\n");
        content.push_str(&format!("chaingraph_buffer_pool_hit_rate {:.4}\n", snapshot.buffer_pool_hit_rate));
        
        content.push_str("# HELP chaingraph_buffer_pool_evictions_total Number of page evictions\n");
        content.push_str("# TYPE chaingraph_buffer_pool_evictions_total counter\n");
        content.push_str(&format!("chaingraph_buffer_pool_evictions_total {}\n", snapshot.buffer_pool_evictions));
        
        // 图操作指标
        content.push_str("# HELP chaingraph_vertices_inserted_total Total vertices inserted\n");
        content.push_str("# TYPE chaingraph_vertices_inserted_total counter\n");
        content.push_str(&format!("chaingraph_vertices_inserted_total {}\n", snapshot.vertices_inserted));
        
        content.push_str("# HELP chaingraph_edges_inserted_total Total edges inserted\n");
        content.push_str("# TYPE chaingraph_edges_inserted_total counter\n");
        content.push_str(&format!("chaingraph_edges_inserted_total {}\n", snapshot.edges_inserted));
        
        // 系统指标
        content.push_str("# HELP chaingraph_uptime_seconds System uptime in seconds\n");
        content.push_str("# TYPE chaingraph_uptime_seconds counter\n");
        content.push_str(&format!("chaingraph_uptime_seconds {}\n", snapshot.uptime_seconds));
        
        PrometheusMetrics { content }
    }

    /// 重置所有指标
    pub fn reset(&self) {
        self.query_stats.total_queries.store(0, Ordering::Relaxed);
        self.query_stats.success_queries.store(0, Ordering::Relaxed);
        self.query_stats.failed_queries.store(0, Ordering::Relaxed);
        self.query_stats.total_duration_us.store(0, Ordering::Relaxed);
        self.query_stats.slow_queries.store(0, Ordering::Relaxed);
        
        self.buffer_pool_stats.hits.store(0, Ordering::Relaxed);
        self.buffer_pool_stats.misses.store(0, Ordering::Relaxed);
        self.buffer_pool_stats.evictions.store(0, Ordering::Relaxed);
        self.buffer_pool_stats.dirty_writes.store(0, Ordering::Relaxed);
        
        self.graph_stats.vertices_inserted.store(0, Ordering::Relaxed);
        self.graph_stats.edges_inserted.store(0, Ordering::Relaxed);
        self.graph_stats.vertices_queried.store(0, Ordering::Relaxed);
        self.graph_stats.edges_queried.store(0, Ordering::Relaxed);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 查询计时器
pub struct QueryTimer {
    start: Instant,
}

impl QueryTimer {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// 全局指标实例
static METRICS: once_cell::sync::Lazy<Arc<Metrics>> = once_cell::sync::Lazy::new(|| {
    Arc::new(Metrics::new())
});

/// 获取全局指标实例
pub fn global_metrics() -> Arc<Metrics> {
    METRICS.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_snapshot() {
        let metrics = Metrics::new();
        
        let timer = metrics.record_query_start();
        std::thread::sleep(Duration::from_millis(10));
        metrics.record_query_complete(timer, true);
        
        metrics.record_buffer_hit();
        metrics.record_buffer_miss();
        metrics.record_vertex_insert();
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_queries, 1);
        assert_eq!(snapshot.success_queries, 1);
        assert!(snapshot.avg_query_duration_ms >= 10.0);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Metrics::new();
        metrics.record_query_start();
        metrics.record_buffer_hit();
        
        let prom = metrics.to_prometheus();
        assert!(prom.content.contains("chaingraph_queries_total"));
        assert!(prom.content.contains("chaingraph_buffer_pool_hits_total"));
    }
}
