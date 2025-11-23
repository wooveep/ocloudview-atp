const express = require('express');
const http = require('http');
const WebSocket = require('ws');
const path = require('path');

// 配置
const PORT = process.env.PORT || 8080;
const WS_PORT = process.env.WS_PORT || 8081;

// 创建 Express 应用
const app = express();

// 提供静态文件服务（测试页面）
app.use(express.static(path.join(__dirname, '../client')));

// HTTP 服务器
const httpServer = http.createServer(app);

// WebSocket 服务器
const wss = new WebSocket.Server({ port: WS_PORT });

// 存储所有连接的客户端
const clients = new Set();

// WebSocket 连接处理
wss.on('connection', (ws, req) => {
    const clientIp = req.socket.remoteAddress;
    console.log(`[WebSocket] 新客户端连接: ${clientIp}`);

    clients.add(ws);

    // 发送欢迎消息
    ws.send(JSON.stringify({
        type: 'welcome',
        message: 'Connected to Guest Agent WebSocket Server',
        timestamp: Date.now()
    }));

    // 接收消息
    ws.on('message', (data) => {
        try {
            const message = JSON.parse(data);
            console.log(`[WebSocket] 收到消息:`, message);

            // 处理不同类型的消息
            switch (message.type) {
                case 'keydown':
                case 'keyup':
                case 'keypress':
                    handleKeyEvent(message);
                    break;

                case 'register':
                    console.log(`[WebSocket] Guest Agent 注册: ${message.agentId}`);
                    break;

                default:
                    console.log(`[WebSocket] 未知消息类型: ${message.type}`);
            }

            // 广播到所有其他客户端（可选）
            // broadcast(data, ws);
        } catch (error) {
            console.error('[WebSocket] 解析消息失败:', error);
        }
    });

    // 处理断开连接
    ws.on('close', () => {
        console.log(`[WebSocket] 客户端断开: ${clientIp}`);
        clients.delete(ws);
    });

    // 处理错误
    ws.on('error', (error) => {
        console.error('[WebSocket] 错误:', error);
    });
});

// 处理键盘事件
function handleKeyEvent(event) {
    console.log(`[KeyEvent] Type: ${event.type}, Key: ${event.key}, Code: ${event.code}, Trusted: ${event.isTrusted}`);

    // TODO: 将事件转发到 Test Controller
    // 这里可以通过另一个 WebSocket 连接或 HTTP 请求发送到控制端
}

// 广播消息到所有客户端（除了发送者）
function broadcast(data, sender) {
    clients.forEach(client => {
        if (client !== sender && client.readyState === WebSocket.OPEN) {
            client.send(data);
        }
    });
}

// 启动 HTTP 服务器
httpServer.listen(PORT, () => {
    console.log(`[HTTP] 服务器运行在 http://localhost:${PORT}`);
    console.log(`[HTTP] 测试页面: http://localhost:${PORT}/test.html`);
});

console.log(`[WebSocket] WebSocket 服务器运行在 ws://localhost:${WS_PORT}`);

// 优雅关闭
process.on('SIGTERM', () => {
    console.log('收到 SIGTERM 信号，关闭服务器...');

    // 关闭所有 WebSocket 连接
    clients.forEach(client => {
        client.close();
    });

    wss.close(() => {
        console.log('WebSocket 服务器已关闭');
    });

    httpServer.close(() => {
        console.log('HTTP 服务器已关闭');
        process.exit(0);
    });
});
