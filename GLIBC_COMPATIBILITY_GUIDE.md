# glibc 版本兼容性问题解决方案

## 🔴 问题描述

在 CentOS 7 或其他老系统上运行便携式 ATP 时，出现 glibc 版本错误：

```
/root/portable-atp/atp: /lib64/libm.so.6: version `GLIBC_2.29' not found
```

**原因**: 编译系统（Ubuntu 24.04, glibc 2.39）的版本高于目标系统（CentOS 7, glibc 2.17）。

## ✅ 解决方案

### 方案 1: 在目标系统上直接编译（最推荐）⭐⭐⭐⭐⭐

**优点**: 完美兼容，无版本问题
**缺点**: 需要在目标系统安装编译工具

#### 步骤：

```bash
# 1. 登录目标服务器
ssh root@ocloud01

# 2. 安装 Rust（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. 安装编译依赖
# CentOS 7/8/Rocky Linux
yum install -y gcc libvirt-devel openssl-devel pkg-config git

# Ubuntu/Debian
apt-get install -y build-essential libvirt-dev libssl-dev pkg-config git

# 4. 克隆或传输代码
git clone <your-repo-url>
# 或者
scp -r /path/to/ocloudview-atp root@ocloud01:/opt/

# 5. 编译
cd ocloudview-atp
cargo build --release --manifest-path atp-application/cli/Cargo.toml

# 6. 二进制文件位置
./atp-application/target/release/atp --version
```

### 方案 2: 使用 Docker 容器编译（适合 CI/CD）⭐⭐⭐⭐

**优点**: 可控的编译环境，适合批量构建
**缺点**: 需要 Docker 环境

#### 使用 CentOS 7 容器编译：

```bash
# 1. 构建编译容器
docker build -f Dockerfile.centos7 -t atp-builder:centos7 .

# 2. 在容器中编译
docker run --rm -v $(pwd):/build atp-builder:centos7 bash -c "
    cd /build && \
    source /root/.cargo/env && \
    cargo build --release --manifest-path atp-application/cli/Cargo.toml
"

# 3. 编译完成后，二进制文件在本地
ls -lh atp-application/target/release/atp
```

#### 使用 Ubuntu 20.04 容器（glibc 2.31）：

```bash
docker run --rm -v $(pwd):/build -w /build rust:1.75 bash -c "
    apt-get update && \
    apt-get install -y libvirt-dev pkg-config && \
    cargo build --release --manifest-path atp-application/cli/Cargo.toml
"
```

### 方案 3: 使用 GitHub Actions 自动构建（最自动化）⭐⭐⭐⭐⭐

创建 `.github/workflows/build.yml`:

```yaml
name: Build ATP for Multiple Platforms

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build-centos7:
    runs-on: ubuntu-latest
    container: centos:7
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          yum install -y centos-release-scl
          yum install -y devtoolset-11 libvirt-devel openssl-devel wget
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

      - name: Build
        run: |
          source $HOME/.cargo/env
          cargo build --release --manifest-path atp-application/cli/Cargo.toml

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: atp-centos7
          path: atp-application/target/release/atp

  build-ubuntu:
    runs-on: ubuntu-20.04
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libvirt-dev pkg-config

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release --manifest-path atp-application/cli/Cargo.toml

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: atp-ubuntu20.04
          path: atp-application/target/release/atp
```

### 方案 4: 使用预编译的静态二进制（实验性）⭐⭐

尝试使用 `musl` 进行静态链接（但 libvirt 绑定可能不支持）：

```bash
# 安装 musl 工具链
rustup target add x86_64-unknown-linux-musl

# 尝试编译（可能失败）
cargo build --release --target x86_64-unknown-linux-musl
```

**注意**: 由于 `virt` crate 依赖系统的 libvirt 动态库，完全静态链接通常不可行。

## 📊 方案对比

| 方案 | 兼容性 | 难度 | 速度 | 推荐度 |
|------|--------|------|------|--------|
| 目标系统编译 | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ | ✅ **首选** |
| Docker CentOS 7 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ✅ **CI/CD** |
| Docker Ubuntu 20 | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | 适合新系统 |
| GitHub Actions | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ **自动化** |
| musl 静态链接 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | 不推荐 |

## 🎯 快速决策

### 如果你只需要在 ocloud01 上运行：
→ **方案 1**：直接在 ocloud01 编译

### 如果需要支持多个老系统：
→ **方案 2**：使用 Docker CentOS 7 容器编译

### 如果需要持续集成/自动发布：
→ **方案 3**：配置 GitHub Actions

## 🔍 检查目标系统信息

在目标服务器上运行：

```bash
# 检查 glibc 版本
ldd --version

# 检查系统版本
cat /etc/os-release

# 检查已安装的 libvirt
rpm -qa | grep libvirt    # CentOS/RHEL
dpkg -l | grep libvirt    # Ubuntu/Debian
```

## 📝 示例：在 ocloud01 上快速部署

```bash
# 一键脚本（在 ocloud01 上运行）
curl -sSL https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
yum install -y gcc libvirt-devel openssl-devel pkg-config

# 下载源码（替换为实际地址）
git clone <repo-url> /opt/ocloudview-atp
cd /opt/ocloudview-atp

# 编译
cargo build --release --manifest-path atp-application/cli/Cargo.toml

# 测试
./atp-application/target/release/atp --version

# 创建链接
ln -s /opt/ocloudview-atp/atp-application/target/release/atp /usr/local/bin/atp
atp vdi verify
```

## ⚠️ 常见问题

### Q: 为什么不能像 Go 那样直接编译出单一二进制？
A: Rust 可以，但 ATP 依赖 libvirt C 库，这个库必须动态链接。libvirt 又依赖许多系统库（XML、SSL、SSH 等），完全静态链接非常困难且不推荐。

### Q: 我应该使用哪个 glibc 版本编译？
A: 使用**最老的目标系统**的 glibc 版本。例如：
- 支持 CentOS 7: 使用 glibc 2.17
- 支持 Ubuntu 18.04: 使用 glibc 2.27
- 支持 Ubuntu 20.04: 使用 glibc 2.31

### Q: 能否同时生成多个版本的二进制？
A: 可以！使用 GitHub Actions 或本地 Docker 同时编译多个版本。

