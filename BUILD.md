# Allay 构建指南

## 环境要求

| 工具 | 最低版本 | 说明 |
|------|---------|------|
| Node.js | >= 24.15.0 | JavaScript 运行时 |
| pnpm | >= 10 | 包管理器 |
| Rust | >= 1.90.0 | Rust 编译工具链 |
| Git | 任意 | 版本控制 |

### 安装 Rust

```powershell
# 安装 Rustup（推荐）
winget install Rustlang.Rustup

# 或访问 https://rustup.rs/
```

## 首次构建

```powershell
# 1. 克隆项目
git clone https://github.com/luolita13/Allay.git
cd Allay

# 2. 安装依赖
pnpm install

# 3. 构建（开发版，带 updater 签名）
cd apps/app
$env:TAURI_SIGNING_PRIVATE_KEY = [System.IO.File]::ReadAllText((Resolve-Path "allay-updater-key"), [System.Text.Encoding]::UTF8).Trim()
pnpm tauri build --config tauri-release.conf.json
```

## 生成签名密钥（仅首次或密钥丢失时）

**私钥文件 `allay-updater-key` 在 `.gitignore` 中，不会提交到仓库。** 如果丢失需要重新生成：

```powershell
cd apps/app
$env:CI = "true"
pnpm tauri signer generate --ci --write-keys allay-updater-key --force
```

这会在当前目录生成两个文件：

| 文件 | 说明 | 是否提交 Git |
|------|------|:-----------:|
| `allay-updater-key` | 私钥（签名用） | ❌ 已在 `.gitignore` |
| `allay-updater-key.pub` | 公钥（验证用） | ✅ 需提交 |

> **重要**：新公钥必须同步更新到 `tauri.conf.json` 和 `tauri-release.conf.json` 的 `plugins.updater.pubkey` 字段。公钥内容就是 `.pub` 文件的全部文本。

## 构建配置说明

### 两个 Tauri 配置文件的区别

| 配置文件 | 用途 | 特点 |
|---------|------|------|
| `tauri.conf.json` | 开发 / 基础构建 | 不生成签名文件 |
| `tauri-release.conf.json` | 发布构建 | 合并基础配置 + 额外启用 updater 签名 |

`tauri-release.conf.json` 相对基础配置额外开启：

```json
{
  "bundle": {
    "createUpdaterArtifacts": "v1Compatible",  // 生成 .sig + .nsis.zip
    "targets": "nsis"                           // 只生成 NSIS，跳过慢速 MSI
  },
  "build": {
    "features": ["updater"]                     // 编译 updater 功能
  }
}
```

### 为什么跳过 MSI？

MSI 构建会调用 Windows Installer 工具链，耗时很长且实际分发场景中 NSIS 安装包更常用。如需 MSI，将 `"targets"` 改为 `"all"` 或 `"msi"` 即可。

## 构建产物

构建完成后产物在 `target/release/bundle/nsis/`：

| 文件 | 说明 |
|------|------|
| `Allay_x.x.x_x64-setup.exe` | NSIS 安装包 |
| `Allay_x.x.x_x64-setup.exe.sig` | 安装包 minisign 签名 |
| `Allay_x.x.x_x64-setup.nsis.zip` | 安装包 ZIP 压缩 |
| `Allay_x.x.x_x64-setup.nsis.zip.sig` | ZIP 包 minisign 签名 |
| `latest.json` | 自动更新元数据 |

可执行文件在 `target/release/Allay.exe`。

## 自动更新原理

1. 构建时，Tauri 用**私钥**对安装包签名，生成 `.sig` 文件
2. App 内置**公钥**（写在 `tauri.conf.json` 的 `plugins.updater.pubkey`）
3. 运行时，App 从 GitHub Releases 下载 `latest.json`，对比版本号
4. 有新版本时下载 `.nsis.zip` 和 `.nsis.zip.sig`
5. 用公钥验证签名 → 校验通过 → 静默安装更新

## 常见问题

### Q: 构建报 "A public key has been found, but no private key"

需要设置 `TAURI_SIGNING_PRIVATE_KEY` 环境变量，指向 `allay-updater-key` 文件内容。

### Q: 私钥文件在哪？

`apps/app/allay-updater-key`，不在 Git 仓库中，需要本地生成或从备份恢复。

### Q: Rust 编译太慢？

首次编译需要下载并编译所有依赖，后续增量编译会快很多。release 模式下 Tauri 编译预计需要 5-10 分钟。
