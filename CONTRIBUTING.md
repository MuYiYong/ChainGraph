# 如何贡献

感谢你对 ChainGraph 的兴趣！我们欢迎各种形式的贡献。

## 开发环境设置

ChainGraph 仅支持容器化部署，开发时也推荐使用 Docker：

```bash
# 克隆仓库
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# 构建开发镜像
docker build -t chaingraph:dev .

# 运行测试
docker run --rm chaingraph:dev cargo test
```

## 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 为新功能编写测试用例
- 保持代码注释清晰

## 测试

所有测试都应该在容器中通过：

```bash
# 构建并测试
docker build -t chaingraph:test .
docker run --rm chaingraph:test cargo test
```

## 提交 PR

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 报告问题

如果发现 bug 或有功能建议，请在 GitHub Issues 中提交。

## 许可证

通过贡献代码，你同意你的贡献将按照 Apache-2.0 许可证授权。
