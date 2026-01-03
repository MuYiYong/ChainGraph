//! HTTP 服务器模块
//!
//! 提供 REST API 和 GQL 查询接口

use crate::algorithm::{EdmondsKarp, PathFinder, TraceDirection};
use crate::error::{Error, Result};
use crate::graph::{EdgeId, GraphCatalog, VertexId};
use crate::metrics;
use crate::query::{GqlParser, QueryExecutor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub catalog: Arc<GraphCatalog>,
}

/// 启动服务器
pub async fn start_server(config: ServerConfig, catalog: Arc<GraphCatalog>) -> Result<()> {
    let state = AppState { catalog };

    let app = Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 指标和统计
        .route("/metrics", get(metrics_handler))
        .route("/stats", get(stats_handler))
        // GQL 查询
        .route("/query", post(execute_query))
        // 顶点操作
        .route("/vertices/:id", get(get_vertex))
        .route("/vertices/address/:address", get(get_vertex_by_address))
        // 边操作
        .route("/edges/:id", get(get_edge))
        .route("/vertices/:id/outgoing", get(get_outgoing_edges))
        .route("/vertices/:id/incoming", get(get_incoming_edges))
        // 图算法
        .route("/algorithm/shortest-path", post(shortest_path))
        .route("/algorithm/all-paths", post(all_paths))
        .route("/algorithm/max-flow", post(max_flow))
        .route("/algorithm/trace", post(trace_path))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    println!("ChainGraph 服务器启动于 http://{}", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| Error::ServerError(format!("绑定地址失败: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::ServerError(format!("服务器错误: {}", e)))?;

    Ok(())
}

// ==================== 处理器 ====================

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Prometheus 格式指标
async fn metrics_handler() -> Response {
    use axum::body::Body;
    
    let metrics = metrics::global_metrics();
    let prom = metrics.to_prometheus();
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(Body::from(prom.content))
        .unwrap()
        .into_response()
}

/// 详细统计信息
async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = metrics::global_metrics();
    let snapshot = metrics.snapshot();
    
    // 获取缓冲池水位信息
    let graph = state.catalog.current_graph();
    let watermark = graph.buffer_pool_watermark();
    
    Json(serde_json::json!({
        "query": {
            "total": snapshot.total_queries,
            "success": snapshot.success_queries,
            "failed": snapshot.failed_queries,
            "avg_duration_ms": snapshot.avg_query_duration_ms,
            "slow_queries": snapshot.slow_queries,
            "qps": snapshot.qps,
        },
        "buffer_pool": {
            "hits": snapshot.buffer_pool_hits,
            "misses": snapshot.buffer_pool_misses,
            "hit_rate": snapshot.buffer_pool_hit_rate,
            "evictions": snapshot.buffer_pool_evictions,
            "dirty_writes": snapshot.buffer_pool_dirty_writes,
            "watermark": watermark,
        },
        "graph": {
            "vertices_inserted": snapshot.vertices_inserted,
            "edges_inserted": snapshot.edges_inserted,
            "vertices_queried": snapshot.vertices_queried,
            "edges_queried": snapshot.edges_queried,
        },
        "system": {
            "uptime_seconds": snapshot.uptime_seconds,
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}

/// GQL 查询请求
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

/// 执行 GQL 查询
async fn execute_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> axum::response::Response {
    let executor = QueryExecutor::new(state.catalog.clone());

    match GqlParser::new(&req.query).parse() {
        Ok(stmt) => match executor.execute(&stmt) {
            Ok(result) => (StatusCode::OK, Json(ApiResponse::success(result))).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(&format!("执行错误: {}", e))),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("解析错误: {}", e))),
        )
            .into_response(),
    }
}

/// 获取顶点
async fn get_vertex(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> axum::response::Response {
    let graph = state.catalog.current_graph();
    match graph.get_vertex(VertexId::new(id)) {
        Some(vertex) => (StatusCode::OK, Json(ApiResponse::success(vertex))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("顶点不存在")),
        )
            .into_response(),
    }
}

/// 通过地址获取顶点
async fn get_vertex_by_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> axum::response::Response {
    // 地址作为普通字符串处理
    let graph = state.catalog.current_graph();
    match graph.get_vertex_by_address(&address) {
        Some(vertex) => (StatusCode::OK, Json(ApiResponse::success(vertex))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("顶点不存在")),
        )
            .into_response(),
    }
}

/// 获取边
async fn get_edge(State(state): State<AppState>, Path(id): Path<u64>) -> axum::response::Response {
    let graph = state.catalog.current_graph();
    match graph.get_edge(EdgeId::new(id)) {
        Some(edge) => (StatusCode::OK, Json(ApiResponse::success(edge))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("边不存在")),
        )
            .into_response(),
    }
}

/// 获取出边
async fn get_outgoing_edges(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let graph = state.catalog.current_graph();
    let edges = graph.get_outgoing_edges(VertexId::new(id));
    (StatusCode::OK, Json(ApiResponse::success(edges)))
}

/// 获取入边
async fn get_incoming_edges(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let graph = state.catalog.current_graph();
    let edges = graph.get_incoming_edges(VertexId::new(id));
    (StatusCode::OK, Json(ApiResponse::success(edges)))
}

/// 路径请求
#[derive(Debug, Deserialize)]
pub struct PathRequest {
    pub source: u64,
    pub target: u64,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_k")]
    pub k: usize,
}

fn default_max_depth() -> usize {
    10
}

fn default_k() -> usize {
    5
}

/// 最短路径
async fn shortest_path(
    State(state): State<AppState>,
    Json(req): Json<PathRequest>,
) -> axum::response::Response {
    let graph = state.catalog.current_graph();
    let finder = PathFinder::new(graph);
    let result = finder.shortest_path(VertexId::new(req.source), VertexId::new(req.target));

    match result {
        Some(path) => (StatusCode::OK, Json(ApiResponse::success(path))).into_response(),
        None => (StatusCode::OK, Json(ApiResponse::<()>::error("路径不存在"))).into_response(),
    }
}

/// 所有路径
async fn all_paths(
    State(state): State<AppState>,
    Json(req): Json<PathRequest>,
) -> impl IntoResponse {
    let graph = state.catalog.current_graph();
    let finder = PathFinder::new(graph);
    let paths = finder.all_paths(
        VertexId::new(req.source),
        VertexId::new(req.target),
        req.max_depth,
    );

    (StatusCode::OK, Json(ApiResponse::success(paths)))
}

/// 最大流请求
#[derive(Debug, Deserialize)]
pub struct MaxFlowRequest {
    pub source: u64,
    pub sink: u64,
}

/// 最大流
async fn max_flow(
    State(state): State<AppState>,
    Json(req): Json<MaxFlowRequest>,
) -> impl IntoResponse {
    let graph = state.catalog.current_graph();
    let algo = EdmondsKarp::new(graph);
    let result = algo.max_flow(VertexId::new(req.source), VertexId::new(req.sink));

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

/// 追踪请求
#[derive(Debug, Deserialize)]
pub struct TraceRequest {
    pub start: u64,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_direction() -> String {
    "forward".to_string()
}

/// 路径追踪
async fn trace_path(
    State(state): State<AppState>,
    Json(req): Json<TraceRequest>,
) -> impl IntoResponse {
    let graph = state.catalog.current_graph();
    let finder = PathFinder::new(graph);

    let direction = match req.direction.as_str() {
        "backward" => TraceDirection::Backward,
        "both" => TraceDirection::Both,
        _ => TraceDirection::Forward,
    };

    let traces = finder.trace(VertexId::new(req.start), direction, req.max_depth, None);

    (StatusCode::OK, Json(ApiResponse::success(traces)))
}

/// 统计信息
/// 图统计信息
#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub vertex_count: usize,
    pub edge_count: usize,
    pub buffer_pool_size: usize,
    pub cached_pages: usize,
}

/// API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
