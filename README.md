# Allay

Minecraft 启动器，基于 Tauri 2 + Vue 3 + Rust。

![Issues](https://img.shields.io/github/issues-raw/luolita13/Allay?color=c78aff&style=for-the-badge)
![Last Commit](https://img.shields.io/github/last-commit/luolita13/Allay?color=c78aff&style=for-the-badge)

## 功能

- CurseForge 集成，内置搜索、浏览和安装模组包
- 国内镜像源：BMCLAPI（游戏文件）、MCIMirror（社区资源）
- 多线程分块下载
- 联机功能 (Game Link)，基于 EasyTier，支持房间聊天
- 崩溃诊断，自动分析日志并给出修复建议
- 主题包系统
- Java 探测（扩展路径 + SHA-1 缓存 + 注册表扫描）
- 自定义游戏窗口标题
- Plugin 分类浏览、Collections 合集、Gallery 视图
- 实例导出（.mrpack / .zip）
- 实验性功能面板

## 开发

```bash
$env:SQLX_OFFLINE='true'

pnpm app:dev    # 开发
pnpm app:build  # 构建
```

## 许可

各包使用各自的许可证，详见各包内 LICENSE 文件。
