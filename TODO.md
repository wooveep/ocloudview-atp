# OCloudView ATP - 开发任务清单

## 项目状态总览

| 指标 | 当前值 | 目标值 |
|------|--------|--------|
| 整体进度 | **92%** | 100% |
| 代码行数 | 32,500+ | - |
| 测试用例 | **98** | 200+ |
| 测试覆盖率 | **78%** | 80%+ |
| 文档数量 | 44 | - |

**当前版本**: v0.5.1-dev
**最后更新**: 2026-01-16

---

## 🔥 剩余功能优先级

### P0 - 阻塞发布 (本周)

| 任务 | 模块 | 位置 | 说明 |
|------|------|------|------|
| SSH Host Key 验证 | ssh-executor | client.rs | ✅ 可通过 atp.toml `[ssh].verify_host_key` 配置 |
| MD5 密码哈希 | vdiplatform | client.rs:84 | 弱加密，需服务端配合升级 |

### P1 - 高优先级 (本月)

| 任务 | 模块 | 位置 | 说明 |
|------|------|------|------|
| SPICE RSA 认证 | protocol/spice | channel.rs:265 | 实现 RSA-OAEP 密码加密 |
| SPICE TLS 支持 | protocol/spice | client.rs:158 | 加密连接 |
| Custom 协议实现 | protocol | custom.rs:33-51 | 4 个 TODO 待实现 |
| CLI 命令补全 | cli | keyboard.rs, mouse.rs, command.rs | 5 个占位实现待完成 |

### P2 - 中优先级 (下个迭代)

| 任务 | 模块 | 位置 | 说明 |
|------|------|------|------|
| SPICE 输入发送 | protocol/spice | inputs.rs:197-312 | 6 个 TODO - 实际发送逻辑 |
| SPICE 显示处理 | protocol/spice | display.rs | 10 个 TODO - 视频/绘图解码 |
| SPICE USB 重定向 | protocol/spice | usbredir.rs | 4 个 TODO |
| Storage tags 过滤 | storage | repositories/*.rs | 2 个 TODO |
| 性能指标持久化 | transport | manager.rs:192 | 数据库集成 |

### P3 - 低优先级 (后续)

| 任务 | 模块 | 位置 | 说明 |
|------|------|------|------|
| SPICE XML 解析优化 | protocol/spice | discovery.rs:102 | 使用 quick-xml |
| SPICE 能力协商 | protocol/spice | channel.rs:257, types.rs:191 | 完善能力解析 |
| HTTP API | http-api | - | RESTful + WebSocket |
| Web 控制台 | - | - | 前端界面 |

---

## 📋 代码中的 TODO 清单

### 🎯 atp-core/protocol (SPICE - 29 个)

#### channel.rs
- [ ] **[channel.rs:257]** 从能力协商中确定 mini header
- [ ] **[channel.rs:265]** 实现 RSA 加密密码 (关键功能)

#### client.rs
- [ ] **[client.rs:158]** 添加 TLS 支持

#### inputs.rs
- [ ] **[inputs.rs:197]** 重构为内部可变性
- [ ] **[inputs.rs:226]** 实现实际发送 (send_key_up)
- [ ] **[inputs.rs:268]** 实现实际发送 (send_mouse_position)
- [ ] **[inputs.rs:282]** 实现实际发送 (send_mouse_press)
- [ ] **[inputs.rs:297]** 实现实际发送 (send_mouse_release)
- [ ] **[inputs.rs:312]** 实现实际发送 (send_mouse_scroll)

#### display.rs
- [ ] **[display.rs:175]** MSGC_DISPLAY_INIT
- [ ] **[display.rs:285]** 解析完整的流创建消息
- [ ] **[display.rs:346]** 解析视频流数据并解码
- [ ] **[display.rs:377]** VP8 解码
- [ ] **[display.rs:382]** JPEG 解码
- [ ] **[display.rs:386]** H.264 解码
- [ ] **[display.rs:449]** 解析和处理 SPICE 绘图命令
- [ ] **[display.rs:547]** MSGC_DISPLAY_PREFERRED_COMPRESSION
- [ ] **[display.rs:558]** MSGC_DISPLAY_PREFERRED_VIDEO_CODEC_TYPE

#### discovery.rs
- [ ] **[discovery.rs:102]** 使用 quick-xml 改进 XML 解析
- [ ] **[discovery.rs:301]** SPICE 密码设置逻辑
- [ ] **[discovery.rs:383]** 实现备用方法
- [ ] **[discovery.rs:396]** 密码过期时间逻辑

#### usbredir.rs
- [ ] **[usbredir.rs:201]** 发送 USB 重定向协议消息
- [ ] **[usbredir.rs:280]** 发送断开设备消息
- [ ] **[usbredir.rs:341]** 解析 usbredir 协议数据
- [ ] **[usbredir.rs:412]** 使用 libusb 枚举设备

#### types.rs
- [ ] **[types.rs:191]** 解析能力列表

#### mod.rs
- [ ] **[mod.rs:206]** 通过主通道发送原始数据
- [ ] **[mod.rs:217]** 从主通道接收数据

---

### 🎯 atp-core/protocol (Custom - 4 个)

- [ ] **[custom.rs:33]** 实现自定义协议连接逻辑
- [ ] **[custom.rs:41]** 实现发送逻辑
- [ ] **[custom.rs:46]** 实现接收逻辑
- [ ] **[custom.rs:51]** 实现断开逻辑

---

### 🖥️ atp-application/cli (5 个)

- [ ] **[command.rs:24]** 实现实际的命令执行
- [ ] **[keyboard.rs:25]** 实现实际的按键发送
- [ ] **[keyboard.rs:43]** 实现实际的文本发送
- [ ] **[mouse.rs:36]** 实现实际的鼠标点击
- [ ] **[mouse.rs:57]** 实现实际的鼠标移动

---

### 📦 atp-core/storage (2 个)

- [ ] **[scenarios.rs:126]** 支持 tags 过滤
- [ ] **[reports.rs:146]** 支持 tags 过滤 (需要 JSON 函数)

---

### 🚀 atp-core/transport (1 个)

- [ ] **[manager.rs:192]** 数据库集成 - 性能指标持久化

---

## 模块完成度

| 模块 | 完成度 | 代码行数 | 状态 |
|------|--------|----------|------|
| Transport (传输层) | 85% | ~1,562 | 核心完成 |
| Protocol - QMP | 100% | ~440 | ✅ 完成 |
| Protocol - QGA | 100% | ~500 | ✅ 完成 |
| Protocol - VirtioSerial | 95% | ~653 | ✅ 完成 |
| Protocol - SPICE | 65% | ~4,785 | 🔄 开发中 |
| Executor (执行器) | **98%** | **~3,500** | ✅ VDI集成完成 |
| Storage (存储层) | **95%** | ~1,000 | ✅ 主机/映射完成 |
| VDI Platform | 85% | ~1,100 | ✅ 批量操作完成 |
| Verification Server | **100%** | ~1,195 | ✅ Executor集成完成 |
| Guest Verifier | 80% | ~2,910 | ✅ Linux/Windows完成 |
| CLI | 95% | ~1,200 | ✅ VDI集成完成 |
| HTTP API | 20% | ~300 | 🔄 基础框架 |

---

## TODO/FIXME 统计

| 模块 | TODO | FIXME | 说明 |
|------|------|-------|------|
| protocol (SPICE) | 29 | 0 | SPICE 功能待完善 |
| protocol (Custom) | 4 | 0 | 协议待实现 |
| cli | 5 | 0 | 命令实现待完成 |
| storage | 2 | 0 | tags 过滤 |
| transport | 1 | 0 | 性能指标 |
| **总计** | **41** | **0** |

---

## 版本规划

### v0.5.1 (当前)
- [x] 架构合规性修复 (blocking I/O, unwrap)
- [ ] SSH Host Key 验证
- [ ] SPICE RSA 认证

### v0.6.0 (计划)
- [ ] HTTP API
- [ ] WebSocket 实时推送
- [ ] 性能优化

### v1.0.0 (目标)
- [ ] 生产级稳定性
- [ ] 完整文档
- [ ] Web 控制台

---

## 相关链接

- **文档中心**: [docs/README.md](docs/README.md)
- **架构设计**: [docs/LAYERED_ARCHITECTURE.md](docs/LAYERED_ARCHITECTURE.md)
- **版本历史**: [CHANGELOG.md](CHANGELOG.md)

---

**维护者**: OCloudView ATP Team
**最后扫描**: 2026-01-16
