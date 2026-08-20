# Sidecar 适配器协议 v1

> English version: [SIDECAR_PROTOCOL_V1.md](SIDECAR_PROTOCOL_V1.md)

状态：L2 适配器 sidecar 当前使用的通信协议。

---

## 1. 传输与分帧

Sidecar 进程通过两个字节流与主程序通信（启动式 sidecar 使用
`stdin`/`stdout`）。每条消息按以下格式分帧：

```text
+------------------------+---------------------------+
| 4 字节小端长度         | UTF-8 JSON 载荷           |
| payload length (u32)   |                           |
+------------------------+---------------------------+
```

规则：

- 长度前缀只计算 JSON 载荷字节数，不包含 4 字节头。
- 载荷始终是 UTF-8 JSON。
- v1 用于验证流量。参考 Rust 客户端拒绝超过 64 MiB 的帧；sidecar 不得
  发送更大的帧。
- 二进制大块帧留给未来的 v2。v1 在 JSON 内用 base64 字符串传递二进制
  数据，不用于 128 MB 流式传输。

---

## 2. JSON-RPC 风格消息

所有方法都是主机发往 sidecar 的带 `id` 请求。

请求：

```json
{ "id": 1, "method": "probe", "params": {} }
```

成功响应：

```json
{ "id": 1, "result": { "devices": [] } }
```

错误响应：

```json
{ "id": 1, "error": { "code": -32001, "message": "capability not exposed" } }
```

规则：

- `id` 是 `u64`。响应必须回传请求的 `id`。
- v1 中 `params` 始终是 JSON 对象。
- 服务端主动通知在 v1 中可选、非必需；主机客户端不处理未请求的消息。

---

## 3. 方法

### 3.1 `handshake`

Params：

```json
{
  "name": "vnd.example.spi-programmer",
  "version": "1.0.0",
  "plugin_api": 1,
  "capabilities": {
    "spi": {
      "pins": { "cs": "CS1", "sck": "SCK", "mosi": "MOSI", "miso": "MISO" },
      "max_frame": 4092,
      "max_freq_khz": 60000
    },
    "uart": { "endpoint": "UART1" },
    "i2c": false,
    "gpio": false,
    "vcc_control": { "range_mv": [1800, 3300] },
    "wp_control": false
  }
}
```

- `capabilities` 使用 JSON `snake_case`，与主机 `CapabilitySet` 结构完全
  一致。不存在的可选能力直接省略。
- `plugin_api` 是主机的插件 API 版本。

Result：

```json
{ "protocol_version": 1 }
```

主机校验 `protocol_version == 1`，其他值都判定握手失败。sidecar 可以
在 result 中附带额外字段（name、version、capabilities），v1 主机忽略
它们。

### 3.2 `probe`

Params：`{}`

Result：

```json
{
  "devices": [{ "id": "mock-0", "kind": "spi", "detail": "Mock sidecar adapter" }]
}
```

`id`、`kind`、`detail` 都是字符串。`probe` 是只读操作，不受会话所有权
或 `BUSY` 影响。

### 3.3 `open`

Params：

```json
{ "device_id": "mock-0" }
```

Result：

```json
{ "session_id": "sess-1" }
```

主机把返回的 `session_id` 当作不透明字符串。

### 3.4 `close`

Params：

```json
{ "session_id": "sess-1" }
```

Result：`{}`

关闭未知或已关闭的会话目前返回 `BUSY`；v1 没有 `NOT_OPEN` 错误码。

### 3.5 `execute`

Params：

```json
{ "session_id": "sess-1", "op": { "op": "spi_transact", "write_b64": "nw==", "read_len": 4 } }
```

`op` 是对象，第一个键 `op` 选择具体操作。v1 定义：

#### `spi_transact`

```json
{ "op": "spi_transact", "write_b64": "nw==", "read_len": 4 }
```

- `write_b64`：base64 编码的写出字节。
- `read_len`：需要读回的字节数。

Result：

```json
{ "data_b64": "70AY/w==" }
```

`data_b64` 是 base64 编码的读回字节，解码后长度必须等于 `read_len`。

#### `gpio_set`

```json
{ "op": "gpio_set", "pin": "IO0", "level": true }
```

Result：`{}`

如果 sidecar 在握手能力中没有声明 `gpio`，必须返回
`CAPABILITY_NOT_EXPOSED`。

---

## 4. 错误码

|   代码 | 名称                     | 含义                                        |
| -----: | ------------------------ | ------------------------------------------- |
| -32001 | `CAPABILITY_NOT_EXPOSED` | 操作/能力未被声明或暴露。                   |
| -32002 | `BUSY`                   | 适配器/会话忙，或尚未 open 就执行 execute。 |
| -32601 | `METHOD_NOT_FOUND`       | JSON-RPC：未知方法。                        |
| -32602 | `INVALID_PARAMS`         | JSON-RPC：参数无效或缺失。                  |

主机客户端映射：

- `-32001` -> `CAPABILITY_NOT_EXPOSED: <message>`
- `-32002` -> `BUSY: <message>`
- 分帧和 JSON 协议错误会产生可读的错误字符串。

---

## 5. 参考实现

- 主机客户端：`upt-hal` crate 的 `SidecarClient` 与 `ChildTransport`。
- 参考 sidecar：`upt-devices` crate 的
  `upt_ch34x_sidecar_libusb` / `upt_ch34x_sidecar_dll`。
- 测试用 mock sidecar：`sidecar_mock`。
