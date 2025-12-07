# ChainGraph Docker 使用指南

## 快速开始

### 1. 构建镜像

```bash
docker build -t chaingraph:latest .
```

或使用 Docker Compose:

```bash
docker compose build
```

### 2. 启动服务

**使用 Docker Compose (推荐):**

```bash
# 启动服务器
docker compose up -d

# 查看日志
docker compose logs -f

# 停止服务
docker compose down
```

**使用 Docker 命令:**

```bash
# 创建数据卷
docker volume create chaingraph-data

# 启动服务器
docker run -d \
  --name chaingraph \
  -p 8080:8080 \
  -v chaingraph-data:/data \
  chaingraph:latest
```

### 3. 使用 CLI

```bash
# 使用 Docker Compose
docker compose run --rm chaingraph-cli

# 或直接使用 Docker
docker run -it --rm \
  -v chaingraph-data:/data \
  chaingraph:latest \
  chaingraph-cli -d /data
```

### 4. 导入数据

```bash
# 将数据文件放入 ./import 目录
mkdir -p import
cp your_data.csv import/

# 使用 Docker Compose
docker compose --profile import run --rm chaingraph-import

# 或直接使用 Docker
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/import:/import:ro \
  chaingraph:latest \
  chaingraph-import -d /data -i /import/your_data.csv
```

## API 访问

服务启动后，可通过以下端点访问:

- **健康检查**: http://localhost:8080/health
- **查询接口**: http://localhost:8080/query
- **顶点操作**: http://localhost:8080/vertex/{id}
- **边操作**: http://localhost:8080/edge/{id}

### 示例查询

```bash
# 健康检查
curl http://localhost:8080/health

# 执行 GQL 查询
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10"}'

# 获取图统计
curl http://localhost:8080/stats
```

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别 (debug, info, warn, error) |

## 数据持久化

数据存储在 Docker Volume `chaingraph-data` 中。

```bash
# 查看数据卷
docker volume inspect chaingraph-data

# 备份数据
docker run --rm \
  -v chaingraph-data:/data:ro \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/chaingraph-backup.tar.gz -C /data .

# 恢复数据
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/backup:/backup:ro \
  alpine tar xzf /backup/chaingraph-backup.tar.gz -C /data
```

## 生产部署建议

### Kubernetes 部署

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chaingraph
spec:
  replicas: 1
  selector:
    matchLabels:
      app: chaingraph
  template:
    metadata:
      labels:
        app: chaingraph
    spec:
      containers:
      - name: chaingraph
        image: chaingraph:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          limits:
            memory: "2Gi"
            cpu: "2"
          requests:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: chaingraph-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: chaingraph
spec:
  selector:
    app: chaingraph
  ports:
  - port: 8080
    targetPort: 8080
  type: ClusterIP
```

## 构建多架构镜像

```bash
# 启用 buildx
docker buildx create --use

# 构建并推送多架构镜像
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t your-registry/chaingraph:latest \
  --push .
```
