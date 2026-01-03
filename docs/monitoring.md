# ChainGraph 监控指南

## 概述

ChainGraph 提供完善的性能监控和资源管理功能，可与 Prometheus、Grafana 等监控系统集成。

## 监控端点

### 1. Prometheus 指标端点

**URL**: `GET /metrics`  
**格式**: Prometheus text format (version 0.0.4)

导出符合 Prometheus 标准的指标，可直接被 Prometheus 服务器采集。

### 2. 详细统计端点

**URL**: `GET /stats`  
**格式**: JSON

返回详细的系统统计信息，包括查询、缓冲池、图操作等各方面的指标。

**示例响应**：
```json
{
  "query": {
    "total": 12345,
    "success": 12300,
    "failed": 45,
    "avg_duration_ms": 12.5,
    "slow_queries": 3,
    "qps": 102.3
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
    "uptime_seconds": 86400,
    "version": "0.1.0"
  }
}
```

## 核心指标说明

### 查询指标

| 指标名称 | 类型 | 说明 |
|---------|------|------|
| `chaingraph_queries_total` | Counter | 总查询数 |
| `chaingraph_queries_success_total` | Counter | 成功查询数 |
| `chaingraph_queries_failed_total` | Counter | 失败查询数 |
| `chaingraph_query_duration_avg_ms` | Gauge | 平均查询耗时（毫秒） |
| `chaingraph_slow_queries_total` | Counter | 慢查询数（>1秒） |
| `chaingraph_qps` | Gauge | 每秒查询数 |

### 缓冲池指标

| 指标名称 | 类型 | 说明 |
|---------|------|------|
| `chaingraph_buffer_pool_hits_total` | Counter | 缓存命中数 |
| `chaingraph_buffer_pool_misses_total` | Counter | 缓存未命中数 |
| `chaingraph_buffer_pool_hit_rate` | Gauge | 缓存命中率 (0-1) |
| `chaingraph_buffer_pool_evictions_total` | Counter | 页面驱逐数 |

### 图操作指标

| 指标名称 | 类型 | 说明 |
|---------|------|------|
| `chaingraph_vertices_inserted_total` | Counter | 插入顶点总数 |
| `chaingraph_edges_inserted_total` | Counter | 插入边总数 |

### 系统指标

| 指标名称 | 类型 | 说明 |
|---------|------|------|
| `chaingraph_uptime_seconds` | Counter | 系统运行时间（秒） |

## 资源水位监控

### 缓冲池水位状态

ChainGraph 自动监控缓冲池使用率，并提供三级状态：

- **Normal** (<80%): 正常运行
- **Warning** (80-90%): 建议关注，可能影响性能
- **Critical** (≥90%): 需要立即处理，可能导致频繁换页

### 慢查询监控

系统自动跟踪执行时间超过 1 秒的查询，可通过 `slow_queries` 指标监控。

## Prometheus 集成

### 配置 Prometheus 采集

在 Prometheus 配置文件中添加：

```yaml
scrape_configs:
  - job_name: 'chaingraph'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### 示例告警规则

```yaml
groups:
  - name: chaingraph_alerts
    rules:
      # 缓冲池使用率告警
      - alert: BufferPoolHighUsage
        expr: chaingraph_buffer_pool_hit_rate < 0.8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "ChainGraph 缓冲池命中率低"
          description: "缓冲池命中率低于 80%，当前值: {{ $value }}"
      
      # 慢查询告警
      - alert: TooManySlowQueries
        expr: rate(chaingraph_slow_queries_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "ChainGraph 慢查询过多"
          description: "最近 5 分钟慢查询率: {{ $value }}/秒"
      
      # 查询失败率告警
      - alert: HighQueryFailureRate
        expr: rate(chaingraph_queries_failed_total[5m]) / rate(chaingraph_queries_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "ChainGraph 查询失败率过高"
          description: "查询失败率超过 5%"
```

## Grafana 仪表盘

### 推荐面板

1. **查询吞吐量**: `rate(chaingraph_queries_total[1m])`
2. **查询成功率**: `rate(chaingraph_queries_success_total[1m]) / rate(chaingraph_queries_total[1m])`
3. **平均查询耗时**: `chaingraph_query_duration_avg_ms`
4. **缓冲池命中率**: `chaingraph_buffer_pool_hit_rate`
5. **缓冲池驱逐率**: `rate(chaingraph_buffer_pool_evictions_total[5m])`
6. **慢查询趋势**: `rate(chaingraph_slow_queries_total[5m])`

### 面板模板

```json
{
  "dashboard": {
    "title": "ChainGraph Monitoring",
    "panels": [
      {
        "title": "QPS",
        "targets": [{
          "expr": "rate(chaingraph_queries_total[1m])"
        }]
      },
      {
        "title": "Buffer Pool Hit Rate",
        "targets": [{
          "expr": "chaingraph_buffer_pool_hit_rate"
        }]
      }
    ]
  }
}
```

## 性能基准

### 预期指标范围

- **缓冲池命中率**: ≥ 95% (理想), ≥ 80% (可接受)
- **平均查询耗时**: < 100ms (简单查询), < 1s (复杂查询)
- **QPS**: 取决于硬件和查询复杂度
- **查询失败率**: < 1%

### 性能调优建议

1. **缓冲池命中率低**:
   - 增加缓冲池大小 (启动参数 `--buffer-pool-size`)
   - 优化查询减少随机访问
   - 考虑硬件升级（更快的 SSD）

2. **慢查询过多**:
   - 检查查询是否涉及大规模图遍历
   - 考虑添加索引（未来版本）
   - 优化查询逻辑

3. **频繁页面驱逐**:
   - 增大缓冲池容量
   - 优化数据访问模式
   - 减少并发查询数

## 运维建议

### 日常监控检查

- 每日检查缓冲池命中率
- 监控慢查询趋势
- 跟踪查询失败率
- 观察系统资源使用（CPU、内存、磁盘 I/O）

### 告警设置建议

- **Critical**: 查询失败率 > 5%
- **Warning**: 缓冲池命中率 < 80%
- **Warning**: 慢查询率 > 0.1/秒
- **Info**: 缓冲池使用率 > 80%

## 未来计划

- [ ] 线程池监控
- [ ] 句柄资源监控
- [ ] 分布式追踪集成（OpenTelemetry）
- [ ] 结构化日志（JSON 格式）
- [ ] 自动性能分析和建议
- [ ] 实时查询执行计划可视化
