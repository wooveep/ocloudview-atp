# ATP 便携式二进制文件打包说明

## 📦 已创建的便携式包

### 文件清单

1. **portable-atp/** - 便携式目录（80MB）
   - `atp` - 主程序二进制文件（14MB）
   - `atp.sh` - 启动脚本（推荐使用）
   - `lib/` - 所有依赖库（56个库，67MB）
   - `test.toml.example` - 配置文件模板
   - `README.md` - 使用说明

2. **atp-portable.tar.gz** - 压缩包（32MB）
   - 包含上述所有文件
   - 可直接传输到其他服务器

## 🚀 使用方法

### 方案 1: 使用本地便携式目录

```bash
cd /home/cloudyi/ocloudview-atp/portable-atp
./atp.sh vdi verify
```

### 方案 2: 部署到其他服务器

```bash
# 1. 传输压缩包到目标服务器
scp atp-portable.tar.gz user@target-server:/opt/

# 2. 在目标服务器上解压
ssh user@target-server
cd /opt
tar xzf atp-portable.tar.gz

# 3. 使用
cd portable-atp
./atp.sh --version
./atp.sh vdi verify
```

## 🔧 编译方法总结

本项目使用了以下编译策略来创建便携式二进制文件：

### 1. Release 模式编译
```bash
PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig \
cargo build --release --manifest-path atp-application/cli/Cargo.toml
```

### 2. 依赖库收集
```bash
# 创建目录结构
mkdir -p portable-atp/lib

# 复制二进制文件
cp atp-application/target/release/atp portable-atp/

# 收集所有动态库依赖
ldd atp-application/target/release/atp | grep "=> /" | awk '{print $3}' | \
  xargs -I {} cp {} portable-atp/lib/
```

### 3. 创建启动脚本
```bash
cat > portable-atp/atp.sh << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="${SCRIPT_DIR}/lib:${LD_LIBRARY_PATH}"
exec "${SCRIPT_DIR}/atp" "$@"
EOF
chmod +x portable-atp/atp.sh
```

### 4. 打包
```bash
tar czf atp-portable.tar.gz portable-atp/
```

## 📊 为什么不能完全静态链接？

对于 ATP 项目，完全静态链接（如使用 musl）有以下限制：

1. **libvirt 依赖** - virt crate 依赖系统的 libvirt 动态库
2. **OpenSSL 动态链接** - reqwest 默认使用系统的 OpenSSL
3. **glibc 特性** - 某些系统调用需要 glibc 的动态支持

因此，**打包依赖库的方式是最实用的解决方案**。

## 🎯 其他可选方案

### 方案 A: 使用 Docker 容器

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y libvirt0 libssl3
COPY --from=builder /app/target/release/atp /usr/local/bin/
CMD ["atp"]
```

### 方案 B: 使用 cargo-zigbuild（交叉编译）

```bash
# 安装 cargo-zigbuild
cargo install cargo-zigbuild

# 针对特定 glibc 版本编译
cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.31
```

### 方案 C: 使用 AppImage 格式

```bash
# 使用 linuxdeploy 创建 AppImage
linuxdeploy --executable=atp --appdir=AppDir --output=appimage
```

## ✅ 优势对比

| 方案 | 大小 | 兼容性 | 易用性 | 推荐度 |
|------|------|--------|--------|--------|
| 便携式目录 | 80MB | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ 推荐 |
| Docker 容器 | 200MB+ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | 适合服务器 |
| 静态链接 | 20MB | ⭐⭐ | ⭐⭐⭐⭐ | 不适用 |
| AppImage | 90MB | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 适合桌面 |

## 📝 测试清单

- [x] 编译 release 版本
- [x] 收集所有依赖库（56个）
- [x] 创建启动脚本
- [x] 测试 `--version` 命令
- [x] 测试 `vdi --help` 命令
- [x] 创建配置文件模板
- [x] 编写使用说明文档
- [x] 打包成 tar.gz（32MB）

## 🎉 完成！

便携式 ATP 二进制包已准备就绪，可以部署到任何兼容的 Linux x86_64 系统上运行。

**位置**:
- 目录: `/home/cloudyi/ocloudview-atp/portable-atp/`
- 压缩包: `/home/cloudyi/ocloudview-atp/atp-portable.tar.gz`
