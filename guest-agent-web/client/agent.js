// Guest Agent 客户端脚本

class GuestAgent {
    constructor() {
        // WebSocket 配置
        this.wsUrl = `ws://${window.location.hostname}:8081`;
        this.ws = null;
        this.connected = false;
        this.agentId = this.generateAgentId();

        // UI 元素
        this.statusIndicator = document.getElementById('statusIndicator');
        this.statusText = document.getElementById('statusText');
        this.eventList = document.getElementById('eventList');
        this.textInput = document.getElementById('textInput');
        this.textArea = document.getElementById('textArea');

        // 事件计数器
        this.eventCount = 0;
        this.maxEvents = 50; // 最多显示的事件数

        // 初始化
        this.init();
    }

    generateAgentId() {
        return `agent-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    }

    init() {
        console.log('[Agent] 初始化 Guest Agent...');
        this.connectWebSocket();
        this.setupEventListeners();
    }

    connectWebSocket() {
        console.log(`[Agent] 连接到 WebSocket: ${this.wsUrl}`);

        try {
            this.ws = new WebSocket(this.wsUrl);

            this.ws.onopen = () => {
                console.log('[Agent] WebSocket 已连接');
                this.connected = true;
                this.updateStatus(true);

                // 发送注册消息
                this.sendMessage({
                    type: 'register',
                    agentId: this.agentId,
                    userAgent: navigator.userAgent,
                    timestamp: Date.now()
                });
            };

            this.ws.onmessage = (event) => {
                console.log('[Agent] 收到消息:', event.data);
                try {
                    const message = JSON.parse(event.data);
                    this.handleMessage(message);
                } catch (error) {
                    console.error('[Agent] 解析消息失败:', error);
                }
            };

            this.ws.onerror = (error) => {
                console.error('[Agent] WebSocket 错误:', error);
                this.updateStatus(false, '连接错误');
            };

            this.ws.onclose = () => {
                console.log('[Agent] WebSocket 已断开');
                this.connected = false;
                this.updateStatus(false, '连接已断开');

                // 5 秒后重连
                setTimeout(() => {
                    console.log('[Agent] 尝试重新连接...');
                    this.connectWebSocket();
                }, 5000);
            };
        } catch (error) {
            console.error('[Agent] 创建 WebSocket 失败:', error);
            this.updateStatus(false, '连接失败');
        }
    }

    sendMessage(message) {
        if (this.connected && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        } else {
            console.warn('[Agent] WebSocket 未连接，无法发送消息');
        }
    }

    handleMessage(message) {
        switch (message.type) {
            case 'welcome':
                console.log('[Agent] 收到欢迎消息:', message.message);
                break;

            case 'command':
                console.log('[Agent] 收到命令:', message.command);
                // TODO: 处理来自控制端的命令
                break;

            default:
                console.log('[Agent] 未知消息类型:', message.type);
        }
    }

    updateStatus(connected, text = null) {
        if (connected) {
            this.statusIndicator.classList.add('connected');
            this.statusText.textContent = text || '已连接';
        } else {
            this.statusIndicator.classList.remove('connected');
            this.statusText.textContent = text || '未连接';
        }
    }

    setupEventListeners() {
        // 监听 keydown 事件
        document.addEventListener('keydown', (event) => {
            this.handleKeyEvent('keydown', event);
        });

        // 监听 keyup 事件
        document.addEventListener('keyup', (event) => {
            this.handleKeyEvent('keyup', event);
        });

        // 监听 keypress 事件（已废弃，但仍然监听以确保兼容性）
        document.addEventListener('keypress', (event) => {
            this.handleKeyEvent('keypress', event);
        });
    }

    handleKeyEvent(eventType, event) {
        // 构建事件数据包
        const eventData = {
            type: eventType,
            key: event.key,
            code: event.code,
            keyCode: event.keyCode,
            ctrlKey: event.ctrlKey,
            shiftKey: event.shiftKey,
            altKey: event.altKey,
            metaKey: event.metaKey,
            timeStamp: event.timeStamp,
            isTrusted: event.isTrusted,
            repeat: event.repeat
        };

        // 发送到 WebSocket
        this.sendMessage(eventData);

        // 显示在 UI 上
        this.logEvent(eventData);
    }

    logEvent(eventData) {
        this.eventCount++;

        const eventItem = document.createElement('div');
        eventItem.className = 'event-item';

        eventItem.innerHTML = `
            <span class="event-type">${eventData.type}</span>
            Key: <span class="event-key">'${eventData.key}'</span>
            Code: <span class="event-code">${eventData.code}</span>
            ${eventData.shiftKey ? '[Shift]' : ''}
            ${eventData.ctrlKey ? '[Ctrl]' : ''}
            ${eventData.altKey ? '[Alt]' : ''}
            ${eventData.metaKey ? '[Meta]' : ''}
            <span class="event-trusted">Trusted: ${eventData.isTrusted}</span>
        `;

        // 插入到列表顶部
        this.eventList.insertBefore(eventItem, this.eventList.firstChild);

        // 限制显示的事件数量
        while (this.eventList.children.length > this.maxEvents) {
            this.eventList.removeChild(this.eventList.lastChild);
        }
    }
}

// 页面加载完成后初始化 Guest Agent
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        window.guestAgent = new GuestAgent();
    });
} else {
    window.guestAgent = new GuestAgent();
}
