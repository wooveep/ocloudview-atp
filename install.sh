#!/bin/bash
# ATP 一键安装脚本（适用于 CentOS 7/8, Rocky Linux, Ubuntu）
# 在目标服务器上直接运行此脚本

set -e

echo "════════════════════════════════════════════════════════"
echo "  ATP 自动化测试平台 - 一键安装脚本"
echo "════════════════════════════════════════════════════════"
echo ""

# 检测系统类型
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VER=$VERSION_ID
else
    echo "❌ 无法检测系统类型"
    exit 1
fi

echo "📋 检测到系统: $OS $VER"
echo ""

# 检查 Rust 是否已安装
if command -v rustc &> /dev/null; then
    echo "✅ Rust 已安装: $(rustc --version)"
else
    echo "📦 安装 Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
    echo "✅ Rust 安装完成"
fi

echo ""
echo "📦 安装编译依赖..."

# 根据系统类型安装依赖
case $OS in
    centos|rhel|rocky|almalinux)
        sudo yum install -y gcc libvirt-devel openssl-devel pkg-config git make
        ;;
    ubuntu|debian)
        sudo apt-get update
        sudo apt-get install -y build-essential libvirt-dev libssl-dev pkg-config git
        ;;
    *)
        echo "⚠️  未知系统类型: $OS"
        echo "请手动安装: gcc, libvirt-devel, openssl-devel, pkg-config, git"
        exit 1
        ;;
esac

echo "✅ 依赖安装完成"
echo ""

# 设置安装目录
INSTALL_DIR="${INSTALL_DIR:-/opt/ocloudview-atp}"

echo "📂 安装目录: $INSTALL_DIR"

# 如果目录已存在，询问是否覆盖
if [ -d "$INSTALL_DIR" ]; then
    echo "⚠️  目录已存在"
    read -p "是否删除并重新安装? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$INSTALL_DIR"
    else
        echo "取消安装"
        exit 0
    fi
fi

echo ""
echo "📥 下载源代码..."

# 这里需要替换为实际的仓库地址
# 如果没有 git 仓库，可以通过 scp 传输源码包
if [ -n "$REPO_URL" ]; then
    git clone "$REPO_URL" "$INSTALL_DIR"
else
    echo "⚠️  请设置环境变量 REPO_URL 或手动复制源码到 $INSTALL_DIR"
    echo "示例: export REPO_URL=https://github.com/your-org/ocloudview-atp.git"
    echo ""
    echo "或者使用 scp 复制:"
    echo "  scp -r /path/to/ocloudview-atp root@$(hostname):$INSTALL_DIR"
    exit 1
fi

cd "$INSTALL_DIR"

echo ""
echo "🔨 开始编译..."
source $HOME/.cargo/env

PKG_CONFIG_PATH=/usr/lib64/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig \
cargo build --release --manifest-path atp-application/cli/Cargo.toml

echo ""
echo "✅ 编译完成！"
echo ""

# 创建软链接
BINARY_PATH="$INSTALL_DIR/atp-application/target/release/atp"
if [ -f "$BINARY_PATH" ]; then
    echo "📦 创建系统链接..."
    sudo ln -sf "$BINARY_PATH" /usr/local/bin/atp
    echo "✅ 已创建 /usr/local/bin/atp"
fi

echo ""
echo "🎉 安装完成！"
echo ""
echo "════════════════════════════════════════════════════════"
echo "  使用方法"
echo "════════════════════════════════════════════════════════"
echo ""
echo "1. 复制配置文件模板:"
echo "   cp $INSTALL_DIR/test.toml.example $INSTALL_DIR/test.toml"
echo ""
echo "2. 编辑配置:"
echo "   vim $INSTALL_DIR/test.toml"
echo ""
echo "3. 运行验证:"
echo "   cd $INSTALL_DIR"
echo "   atp vdi verify"
echo ""
echo "或者:"
echo "   $BINARY_PATH vdi verify"
echo ""
echo "════════════════════════════════════════════════════════"
