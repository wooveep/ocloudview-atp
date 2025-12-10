# VDI 平台 API 探测结果

## 探测时间
2025-12-08

## VDI 平台信息
- **地址**: http://192.168.41.51:8088
- **系统名称**: 云桌面管理系统
- **API 版本**: v1
- **Swagger 文档**: http://192.168.41.51:8088/doc.html

## 发现的主要 API 接口

### 1. 用户登录模块 (客户端登录)

#### 登录接口
- **路径**: `/ocloud/usermodule/login`
- **方法**: POST
- **参数**:
  ```json
  {
    "username": "string",
    "password": "string"
  }
  ```

### 2. 主机管理 API

#### 获取主机列表
- **路径**: `/ocloud/v1/host`
- **方法**: GET
- **参数**:
  - pageNum: 页码 (可选)
  - pageSize: 每页记录数 (可选)
  - Token: 访问令牌 (header)

#### 其他主机操作
- `/ocloud/v1/host/storage` - 获取主机存储信息
- `/ocloud/v1/host/vm` - 获取主机下的虚拟机
- `/ocloud/v1/host/network` - 获取主机网络信息

### 3. 虚拟机(Domain)管理 API

#### 获取虚拟机列表
- **路径**: `/ocloud/v1/domain`
- **方法**: GET
- **参数**:
  - pageNum, pageSize: 分页参数
  - deskpoolId: 桌面池ID (可选)
  - Token: 访问令牌 (header)

#### 虚拟机操作
- `/ocloud/v1/domain/close` - 关闭虚拟机
- `/ocloud/v1/domain/delete` - 删除虚拟机
- `/ocloud/v1/domain/restart` - 重启虚拟机
- `/ocloud/v1/domain/freeze` - 冻结虚拟机

### 4. 桌面池(DeskPool)管理 API

#### 获取桌面池列表
- **路径**: `/ocloud/v1/desk-pool`
- **方法**: GET
- **参数**: pageNum, pageSize, Token

#### 桌面池操作
- `/ocloud/v1/desk-pool/create` - 创建桌面池
- `/ocloud/v1/desk-pool/{id}` - 查询/修改/删除桌面池
- `/ocloud/v1/desk-pool/{id}/domain/list` - 获取桌面池下的虚拟机

### 5. 管理员用户登录

#### 管理员登录
- **路径**: `/ocloud/v1/admin-user/login`
- **方法**: POST
- **参数**:
  ```json
  {
    "username": "string",
    "password": "string"
  }
  ```

## 认证方式

根据 Swagger 文档，该系统使用 **Token 认证**：
1. 先调用登录接口获取 Token
2. 后续请求在 Header 中携带 Token
   ```
   Token: <access_token>
   ```

## 完整 API 文档

完整的 Swagger API 文档已保存到:
- [docs/vdi_swagger_api.json](vdi_swagger_api.json)
- API 总数: 约 400+ 个接口
- 文档行数: 37284 行

## 使用建议

### 1. 测试登录
```bash
curl -X POST "http://192.168.41.51:8088/ocloud/usermodule/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "11111111"
  }'
```

### 2. 获取主机列表
```bash
# 先登录获取 Token
TOKEN="<从登录接口获取>"

# 获取主机列表
curl -X GET "http://192.168.41.51:8088/ocloud/v1/host" \
  -H "Token: $TOKEN"
```

### 3. 获取虚拟机列表
```bash
curl -X GET "http://192.168.41.51:8088/ocloud/v1/domain?pageNum=1&pageSize=10" \
  -H "Token: $TOKEN"
```

## 下一步行动

1. ✅ 已完成 VDI API 探测
2. ✅ 已获取完整 Swagger 文档
3. 🔄 需要更新 `test_connectivity.rs` 使用实际的 API 路径
4. 🔄 需要实现 Token 认证流程
5. 🔄 需要更新 `atp-vdiplatform` 客户端代码

---

**文档创建时间**: 2025-12-08
**维护者**: OCloudView ATP Team
