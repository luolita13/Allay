# Allay 构建指南

## 前置条件

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- [rsign2](https://crates.io/crates/rsign2) — 签名工具 (`cargo install rsign2`)
- Windows 系统（NSIS 构建需要 Windows）

## 构建步骤

### 方式一：一键构建（推荐）

```powershell
.\build.ps1
```

脚本会自动完成以下步骤：
1. 运行 `pnpm tauri build --config tauri-release.conf.json` 编译 NSIS 安装包
2. 解码 base64 包装的私钥为明文格式
3. 使用 `rsign2` 对 `.exe` 和 `.nsis.zip` 分别签名
4. 生成 `latest.json` 更新元数据
5. 验证所有签名

### 方式二：手动构建

#### 1. 安装依赖

```bash
pnpm install
cargo install rsign2
```

#### 2. 修改版本号（发版前必须改两个文件）

| 文件 | 字段 | 示例 |
|------|------|------|
| `apps/app-frontend/package.json` | `"version"` | `"0.12.0"` |
| `apps/app/Cargo.toml` | `version` | `0.12.0` |

#### 3. 构建 NSIS 安装包

```bash
cd apps/app
pnpm tauri build --config tauri-release.conf.json
```

#### 4. 签名

Tauri CLI 的密钥文件使用 base64 包装格式，需要先解码为明文后才能用 rsign2 签名：

```powershell
# 解码私钥
$keyB64 = Get-Content "apps/app/allay-updater-key" -Raw
$keyPlain = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($keyB64))

# 签名 nsis.zip（更新包）
$keyPlain | rsign sign -W -x "target/release/bundle/nsis/Allay_x.x.x_x64-setup.nsis.zip.sig" "target/release/bundle/nsis/Allay_x.x.x_x64-setup.nsis.zip"

# 签名 exe（安装包）
$keyPlain | rsign sign -W -x "target/release/bundle/nsis/Allay_x.x.x_x64-setup.exe.sig" "target/release/bundle/nsis/Allay_x.x.x_x64-setup.exe"
```

## 产物位置

```
target/release/bundle/nsis/
├── Allay_0.12.0_x64-setup.exe          # 安装包
├── Allay_0.12.0_x64-setup.exe.sig      # 安装包签名
├── Allay_0.12.0_x64-setup.nsis.zip     # 更新包（ZIP）
├── Allay_0.12.0_x64-setup.nsis.zip.sig # 更新包签名
└── latest.json                         # 更新元数据
```

## 密钥说明

- 私钥 `apps/app/allay-updater-key` — base64 包装格式，本地保管，**不要提交到 Git**
- 公钥 `apps/app/allay-updater-key.pub` — base64 包装格式，已提交，写死在应用内用于验证更新包签名
- 密钥使用 Tauri CLI 的 `pnpm tauri signer generate -w <path>` 生成
- 注意：Tauri CLI 内部使用 rsign 格式，与独立 minisign 工具不兼容
- `latest.json` — 构建产物，上传到 GitHub Releases 即可，**不需要提交到 Git**

## 发布流程

1. 改版本号（两个文件）
2. 运行 `.\build.ps1`（或手动执行上述步骤）
3. 将 `latest.json` 和 4 个安装包文件上传到 GitHub Releases
