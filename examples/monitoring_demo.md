# ChainGraph 性能监控示例

本文档演示如何使用 ChainGraph 的性能监控功能。

## 启动服务

首先启动 ChainGraph 服务器：

```bash
cd /path/to/ChainGraph
cargo run --release --bin chaingraph-server
```

服务默认运行在 `http://localhost:8080`

## 1. 查看健康状态

```bash
curl http://localhost:8080/health
```

**响应**:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## 2. 查看 Prometheus 指标

```bash
curl http://localhost:8080/metrics
```

**响应示例**:
```
# HELP chaingraph_queries_total Total number of queries
# TYPE chaingraph_queries_total counter
chaingraph_queries_total 1234

# HELP chaingraph_queries_success_total Number of successful queries
# TYPE chaingraph_queries_success_total counter
chaingraph_queries_success_total 1200

# HELP chaingraph_queries_failed_total Number of failed queries
# TYPE chaingraph_queries_failed_total counter
chaingraph_queries_failed_total 34

# HELP chaingraph_query_duration_avg_ms Average query duration in milliseconds
# TYPE chaingraph_query_duration_avg_ms gauge
chaingraph_query_duration_avg_ms 15.23

# HELP chaingraph_slow_queries_total Number of slow queries (>1s)
# TYPE chaingraph_slow_queries_total counter
chaingraph_slow_queries_total 5

# HELP chaingraph_qps Queries per second
# TYPE chaingraph_qps gauge
chaingraph_qps 102.45

# HELP chaingraph_buffer_pool_hits_total Buffer pool cache hits
# TYPE chaingraph_buffer_pool_hits_total counter
chaingraph_buffer_pool_hits_total 98765

# HELP chaingraph_buffer_pool_misses_total Buffer pool cache misses
# TYPE chaingraph_buffer_pool_misses_total counter
chaingraph_buffer_pool_misses_total 1234

# HELP chaingraph_buffer_pool_hit_rate Buffer pool hit rate (0-1)
# TYPE chaingraph_buffer_pool_hit_rate gauge
chaingraph_buffer_pool_hit_rate 0.9876

# HELP chaingraph_buffer_pool_evictions_total Number of page evictions
# TYPE chaingraph_buffer_pool_evictions_total counter
chaingraph_buffer_pool_evictions_total 234

# HELP chaingraph_vertices_inserted_total Total vertices inserted
# TYPE chaingraph_vertices_inserted_total counter
chaingraph_vertices_inserted_total 50000

# HELP chaingraph_edges_inserted_total Total edges inserted
# TYPE chaingraph_edges_inserted_total counter
chaingraph_edges_inserted_total 150000

# HELP chaingraph_uptime_seconds System uptime in seconds
# TYPE chaingraph_uptime_seconds counter
chaingraph_uptime_seconds 3600
```

## 3. 查看详细统计

```bash
curl http://localhost:8080/stats | jq
```

**响应示例**:
```json
{
  "query": {
    "total": 1234,
    "success": 1200,
    "failed": 34,
    "avg_duration_ms": 15.23,
    "slow_queries": 5,
    "qps": 102.45
  },
  "buffer_pool": {
    "hits": 98765,
    "misses": 1234,
    "hit_rate": 0.9876,
    "evictions": 234,
    "dirty_writes": 456,
    "watermark": {
      "cached_pages": 800,
      "total_pages": 1000,
      "usage_percent": 80.0,
      "status": "Warning"
    }
  },
  "graph": {
    "vertices_inserted": 50000,
    "edges_inserted": 150000,
    "vertices_queried": 25000,
    "edges_queried": 75000
  },
  "system": {
    "uptime_seconds": 3600,
    "version": "0.1.0"
  }
}
```

## 4. 执行测试查询并观察指标

### 4.1 创建测试图

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "CREATE GRAPH test_metrics { NODE Account { address String PRIMARY KEY }, EDGE Transfer (Account)-[{ amount int }]->(Account) }; USE GRAPH test_metrics;"
  }'
```

### 4.2 插入测试数据

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "INSERT (:Account {address: \"0x1234\"}), (:Account {address: \"0x5678\"});"
  }'
```

### 4.3 执行多次查询

```bash
# 执行 100 次查询
for i in {1..100}; do
  curl -s -X POST http://localhost:8080/query \
    -H "Content-Type: application/json" \
    -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10;"}' > /dev/null
  echo "Query $i completed"
done
```

### 4.4 查看更新后的指标

```bash
curl http://localhost:8080/stats | jq '.query'
```

**预期输出**:
```json
{
  "total": 102,
  "success": 102,
  "failed": 0,
  "avg_duration_ms": 8.5,
  "slow_queries": 0,
  "qps": 125.6
}
```

## 5. 监控缓冲池水位

### 5.1 检查当前水位

```bash
curl http://localhost:8080/stats | jq '.buffer_pool.watermark'
```

**正常状态** (<80%):
```json
{
  "cached_pages": 650,
  "total_pages": 1000,
  "usage_percent": 65.0,
  "status": "Normal"
}
```

**警告状态** (80-90%):
```json
{
  "cached_pages": 850,
  "total_pages": 1000,
  "usage_percent": 85.0,
  "status": "Warning"
}
```

**危险状态** (≥90%):
```json
{
  "cached_pages": 920,
  "total_pages": 1000,
  "usage_percent": 92.0,
  "status": "Critical"
}
```

### 5.2 观察缓冲池命中率

```bash
curl http://localhost:8080/stats | jq '{hit_rate: .buffer_pool.hit_rate, evictions: .buffer_pool.evictions}'
```

**健康系统** (命中率高):
```json
{
  "hit_rate": 0.985,
  "evictions": 120
}
```

**性能降级** (命中率低):
```json
{
  "hit_rate": 0.65,
  "evictions": 8500
}
```

## 6. 集成 Prometheus

### 6.1 配置 Prometheus

创建 `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'chaingraph'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### 6.2 启动 Prometheus

```bash
docker run -d \
  -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus
```

### 6.3 访问 Prometheus

在浏览器中打开 `http://localhost:9090`，可以查询指标：

- `chaingraph_qps` - 查询吞吐量
- `rate(chaingraph_queries_total[1m])` - 每分钟查询增长率
- `chaingraph_buffer_pool_hit_rate` - 缓冲池命中率

## 7. 使用 Grafana 可视化

### 7.1 启动 Grafana

```bash
docker run -d \
  -p 3000:3000 \
  grafana/grafana
```

### 7.2 添加 Prometheus 数据源

1. 访问 `http://localhost:3000` (默认账号: admin/admin)
2. 添加 Prometheus 数据源: `http://prometheus:9090`

### 7.3 创建仪表盘

常用面板查询：

**查询吞吐量**:
```promql
rate(chaingraph_queries_total[1m])
```

**查询成功率**:
```promql
rate(chaingraph_queries_success_total[1m]) / rate(chaingraph_queries_total[1m])
```

**缓冲池命中率**:
```promql
chaingraph_buffer_pool_hit_rate
```

**慢查询率**:
```promql
rate(chaingraph_slow_queries_total[5m])
```

## 8. 性能调优建议

### 8.1 命中率低 (<80%)

```bash
# 增大缓冲池（重启服务时指定）
chaingraph-server --buffer-pool-size 2000
```

### 8.2 慢查询多

检查慢查询统计：
```bash
curl http://localhost:8080/stats | jq '.query.slow_queries'
```

优化建议：
- 减少图遍历深度
- 使用 LIMIT 限制返回结果
- 考虑添加索引（未来版本）

### 8.3 查询失败率高

```bash
curl http://localhost:8080/stats | jq '{success: .query.success, failed: .query.failed}'
```

检查失败原因：
- 查看服务器日志
- 验证 GQL 语法
- 检查数据完整性

## 9. 压力测试

使用 `wrk` 进行压力测试：

```bash
# 安装 wrk
brew install wrk  # macOS
# apt install wrk  # Ubuntu

# 准备查询负载
cat > query.lua << 'EOF'
wrk.method = "POST"
wrk.body   = '{"query": "MATCH (n:Account) RETURN n LIMIT 10;"}'
wrk.headers["Content-Type"] = "application/json"
EOF

# 执行压测：10 个线程，100 个连接，持续 30 秒
wrk -t10 -c100 -d30s -s query.lua http://localhost:8080/query

# 查看压测后的指标
curl http://localhost:8080/stats | jq
```

## 10. 告警示例

### Prometheus 告警规则

创建 `alerts.yml`:

```yaml
groups:
  - name: chaingraph_alerts
    rules:
      - alert: LowBufferPoolHitRate
        expr: chaingraph_buffer_pool_hit_rate < 0.8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "ChainGraph 缓冲池命中率低"
          description: "命中率: {{ $value | humanizePercentage }}"
      
      - alert: HighQueryFailureRate
        expr: rate(chaingraph_queries_failed_total[5m]) / rate(chaingraph_queries_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "查询失败率过高"
          description: "失败率: {{ $value | humanizePercentage }}"
```

## 总结

ChainGraph 的监控功能提供了：

✅ **实时性能指标** - QPS、响应时间、成功率  
✅ **资源使用监控** - 缓冲池命中率、水位状态  
✅ **Prometheus 集成** - 标准指标格式  
✅ **灵活的 API** - JSON 和文本格式  
✅ **开箱即用** - 无需额外配置

更多详情请参阅 [监控指南](../docs/monitoring.md)。
