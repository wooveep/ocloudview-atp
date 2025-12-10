# VDI 平台登录 API 使用指南

## 概述

本文档说明如何正确调用 VDI 平台的登录 API，包括请求格式、密码加密和响应处理。

## API 端点

```
POST http://192.168.41.51:8088/ocloud/usermodule/login
```

## 请求格式

### HTTP Headers

```
Content-Type: application/json
```

### 请求体 (JSON)

**实际使用的格式**（已验证）：

```json
{
  "username": "用户名",
  "password": "MD5加密的密码"
}
```

**重要字段说明**:
- `username`: 用户账号名
- `password`: **必须** 进行 MD5 加密（小写十六进制格式）

### 可选字段

根据 Swagger API 定义，LoginReq 还支持以下可选字段：

```json
{
  "username": "admin",
  "password": "1bbd886460827015e5d605ed44252251",
  "sAMAccountName": "AD账号",
  "phone": "手机号",
  "idNumber": "身份证号",
  "ipAddr": "客户端IP",
  "macAddr": "MAC地址",
  "software": "客户端软件",
  "systemVersion": "系统版本",
  "terminalType": "终端类型",
  "smsCode": "短信验证码",
  "smsKey": "短信密钥",
  "action": "login"
}
```

## 密码加密

### 为什么需要 MD5 加密？

VDI 平台要求密码必须进行 MD5 加密后再发送，使用小写十六进制格式。

### 加密示例

**原始密码**: `11111111`

**MD5 加密后**: `1bbd886460827015e5d605ed44252251`

### 各语言实现

#### Rust
```rust
use md5;

let password = "11111111";
let password_md5 = format!("{:x}", md5::compute(password.as_bytes()));
// password_md5 = "1bbd886460827015e5d605ed44252251"
```

#### Python
```python
import hashlib

password = "11111111"
password_md5 = hashlib.md5(password.encode()).hexdigest()
# password_md5 = "1bbd886460827015e5d605ed44252251"
```

#### JavaScript/Node.js
```javascript
const crypto = require('crypto');

const password = "11111111";
const password_md5 = crypto.createHash('md5').update(password).digest('hex');
// password_md5 = "1bbd886460827015e5d605ed44252251"
```

#### Shell (curl)
```bash
PASSWORD=$(echo -n "11111111" | md5sum | awk '{print $1}')
# PASSWORD = "1bbd886460827015e5d605ed44252251"

curl -X POST http://192.168.41.51:8088/ocloud/usermodule/login \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"admin\",\"password\":\"$PASSWORD\"}"
```

## 响应格式

### 成功响应

```json
{
  "status": 200,
  "msg": "登录成功",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "userName": "admin",
    "phone": "13800138000",
    "isFirstLogin": 0,
    "needSmsVerify": false,
    "strongPwd": "1",
    "licenseExpireDays": 365,
    "domains": [...],
    "linkingHosts": [...]
  }
}
```

**关键字段**:
- `data.token`: 认证令牌，用于后续 API 请求
- `data.userName`: 用户名
- `data.domains`: 可访问的虚拟机列表
- `data.linkingHosts`: 关联的主机列表

### 失败响应

#### 用户不存在
```json
{
  "msg": "用户不存在",
  "returnCode": 5098,
  "status": 5098
}
```

#### 密码错误
```json
{
  "msg": "密码错误",
  "returnCode": 5099,
  "status": 5099
}
```

#### 其他错误
- `5100`: 账号已被锁定
- `5101`: 账号已过期
- `5102`: 需要短信验证

## 使用示例

### 完整的 Rust 示例

```rust
use reqwest;
use serde_json::{json, Value};
use md5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 准备登录信息
    let base_url = "http://192.168.41.51:8088";
    let username = "admin";
    let password = "11111111";

    // 2. MD5 加密密码
    let password_md5 = format!("{:x}", md5::compute(password.as_bytes()));

    // 3. 构建登录请求
    let login_url = format!("{}/ocloud/usermodule/login", base_url);
    let login_data = json!({
        "username": username,
        "password": password_md5,
    });

    // 4. 发送请求
    let client = reqwest::Client::new();
    let response = client
        .post(&login_url)
        .json(&login_data)
        .send()
        .await?;

    // 5. 处理响应
    if response.status().is_success() {
        let json: Value = response.json().await?;

        if let Some(token) = json["data"]["token"].as_str() {
            println!("登录成功！Token: {}", token);

            // 6. 使用 Token 调用其他 API
            let host_url = format!("{}/ocloud/v1/host", base_url);
            let host_response = client
                .get(&host_url)
                .header("Token", token)
                .send()
                .await?;

            println!("主机列表: {:?}", host_response.text().await?);
        } else {
            eprintln!("登录失败: {}", json["msg"]);
        }
    }

    Ok(())
}
```

### curl 示例

```bash
#!/bin/bash

# VDI 平台地址
BASE_URL="http://192.168.41.51:8088"

# 用户凭据
USERNAME="admin"
PASSWORD="11111111"

# MD5 加密密码
PASSWORD_MD5=$(echo -n "$PASSWORD" | md5sum | awk '{print $1}')

# 登录获取 Token
RESPONSE=$(curl -s -X POST "$BASE_URL/ocloud/usermodule/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD_MD5\"}")

# 提取 Token
TOKEN=$(echo "$RESPONSE" | jq -r '.data.token')

if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
    echo "登录成功！Token: $TOKEN"

    # 使用 Token 获取主机列表
    curl -s "$BASE_URL/ocloud/v1/host" \
      -H "Token: $TOKEN" | jq .
else
    echo "登录失败："
    echo "$RESPONSE" | jq .
fi
```

## Token 使用

获取 Token 后，所有需要认证的 API 请求都需要在 HTTP Header 中携带：

```
Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 示例请求

```bash
curl http://192.168.41.51:8088/ocloud/v1/host \
  -H "Token: YOUR_TOKEN_HERE"
```

### Token 失效处理

如果 Token 失效，API 会返回：

```json
{
  "msg": "token校验失败",
  "status": 1006
}
```

此时需要重新登录获取新的 Token。

## 故障排查

### 问题 1: "用户不存在" (returnCode: 5098)

**原因**: VDI 平台中没有该用户账号

**解决方法**:
1. 登录 VDI 管理后台: http://192.168.41.51:8088
2. 进入"用户管理"模块
3. 创建测试用户或确认已有用户的用户名
4. 更新配置文件中的用户名

### 问题 2: "密码错误" (returnCode: 5099)

**原因**: 密码不正确或加密错误

**解决方法**:
1. 确认密码是否正确
2. 确认密码是否进行了 MD5 加密
3. 确认 MD5 格式为小写十六进制（32个字符）

### 问题 3: "token校验失败" (status: 1006)

**原因**: Token 无效、过期或未提供

**解决方法**:
1. 确认登录成功并获取了有效 Token
2. 确认请求 Header 中正确设置了 Token
3. 如果 Token 过期，重新登录获取新 Token

### 问题 4: 密码加密方法错误

**症状**: 一直提示"用户不存在"或"密码错误"

**解决方法**:
- 确认使用 MD5 加密，不是 Base64 编码
- 验证 MD5 结果：`echo -n "11111111" | md5sum` 应该返回 `1bbd886460827015e5d605ed44252251`

## 测试工具

### 调试工具

项目提供了专门的登录调试工具：

```bash
cd /home/cloudyi/ocloudview-atp
cargo run --example test_vdi_login_debug --manifest-path atp-core/executor/Cargo.toml
```

**功能**:
- 测试多种登录字段组合
- 显示完整的请求和响应
- 提供调试建议

### 完整 API 测试

```bash
cargo run --example test_real_vdi_api --manifest-path atp-core/executor/Cargo.toml
```

**功能**:
- 测试登录认证
- 测试主机列表 API
- 测试虚拟机列表 API
- 测试桌面池列表 API

## API 文档参考

完整的 VDI API 文档:
- Swagger UI: http://192.168.41.51:8088/doc.html#/home
- JSON 文档: [docs/vdi_swagger_api.json](vdi_swagger_api.json)

## 相关文档

- [VDI 连通性测试总结](../VDI_CONNECTIVITY_TEST_SUMMARY.md)
- [VDI API 发现文档](VDI_API_DISCOVERY.md)
- [连通性测试指南](CONNECTIVITY_TEST_GUIDE.md)

---

**创建日期**: 2025-12-08
**最后更新**: 2025-12-08
**维护者**: OCloudView ATP Team
