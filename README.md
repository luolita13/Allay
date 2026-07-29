# Allay

基于 [Modrinth App](https://github.com/modrinth/code) 的定制版 Minecraft 启动器，增加了多项增强功能。

![Issues](https://img.shields.io/github/issues-raw/luolita13/Allay?color=c78aff&style=for-the-badge)
![Last Commit](https://img.shields.io/github/last-commit/luolita13/Allay?color=c78aff&style=for-the-badge)

## 增强功能

- **CurseForge 集成** — 内置搜索、浏览和安装 CurseForge 模组包，无需额外配置 API Key
- **国内镜像源** — 游戏文件下载支持 BMCLAPI 镜像，社区资源下载支持 MCIMirror CDN，版本列表支持 BMCLAPI
- **多线程分块下载** — 大文件自动分块并行下载，大幅提升下载速度
- **联机功能 (Game Link)** — 基于 EasyTier 的局域网穿透联机，支持房间聊天
- **崩溃诊断** — 游戏崩溃时自动分析日志，匹配 40+ 条诊断规则并给出修复建议
- **主题包系统** — zip 格式主题包，支持自定义背景图、accent color、CSS 变量、字体
- **Java 探测增强** — 扩展扫描路径 + SHA-1 缓存 + 注册表扫描，自动识别系统已安装 Java
- **游戏窗口标题** — 自定义 Minecraft 窗口标题
- **内容发现增强** — 新增 Plugin 分类浏览、Collections 合集页面、Gallery 浏览模式
- **实例导出** — 支持 Modrinth (.mrpack) 和 ZIP (.zip) 两种格式
- **实验性功能面板** — 统一管理所有实验性功能开关（联机、高性能 GPU、世界存档显示等）

## 开发

本项目是 Modrinth Monorepo 的 fork，基于 Tauri 2 + Vue 3 (app-frontend) + Rust (theseus/app-lib)。

```bash
# 设置离线 SQLx 模式
$env:SQLX_OFFLINE='true'

# 开发模式
pnpm app:dev

# 构建
pnpm app:build
```

详细开发文档请参考 [上游仓库](https://github.com/modrinth/code) 和 [Modrinth 文档](https://docs.modrinth.com/contributing/theseus/)。

## 许可证

本仓库中各包遵循各自的许可证分发。详见各包的 LICENSE 文件。原始代码版权归 Modrinth 团队所有。
