# Sidecar Adapter Protocol v1

> 中文版：[SIDECAR_PROTOCOL_V1_CN.md](SIDECAR_PROTOCOL_V1_CN.md)

Status: current wire protocol for L2 adapter sidecars.

---

## 1. Transport and framing

A sidecar process communicates with the host over two byte streams
(`stdin`/`stdout` for a launched process). Each message is framed as:

```text
+------------------------+---------------------------+
| 4 bytes little-endian  | UTF-8 JSON payload       |
| payload length (u32)   |                           |
+------------------------+---------------------------+
```

Rules:

- The length prefix counts only the JSON payload bytes; it does not include
  the 4 header bytes.
- The payload is always UTF-8 JSON.
- v1 is for validation traffic. The reference Rust client caps accepted
  frames at 64 MiB; a sidecar must not send larger frames.
- Binary bulk frames are reserved for a future v2. v1 moves binary data
  inside JSON as base64 strings and is not intended for 128 MB streaming.

---

## 2. JSON-RPC style messages

All methods are host-to-sidecar requests with an `id`.

Request:

```json
{ "id": 1, "method": "probe", "params": {} }
```

Success response:

```json
{ "id": 1, "result": { "devices": [] } }
```

Error response:

```json
{ "id": 1, "error": { "code": -32001, "message": "capability not exposed" } }
```

Rules:

- `id` is a `u64`. Responses must echo the request `id`.
- `params` is always a JSON object in v1.
- Server-originated notifications are optional and not required in v1; the
  host client does not act on unsolicited messages.

---

## 3. Methods

### 3.1 `handshake`

Params:

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

- `capabilities` uses JSON `snake_case` and matches the host
  `CapabilitySet` structures exactly. Absent optional capabilities are
  omitted.
- `plugin_api` is the host plugin API version.

Result:

```json
{ "protocol_version": 1 }
```

The host verifies `protocol_version == 1`; any other value fails the
handshake. A sidecar may include additional result fields (name, version,
capabilities); the host ignores them in v1.

### 3.2 `probe`

Params: `{}`

Result:

```json
{
  "devices": [{ "id": "mock-0", "kind": "spi", "detail": "Mock sidecar adapter" }]
}
```

`id`, `kind`, and `detail` are strings. `probe` is read-only and must not be
affected by session ownership or `BUSY`.

### 3.3 `open`

Params:

```json
{ "device_id": "mock-0" }
```

Result:

```json
{ "session_id": "sess-1" }
```

The host treats the returned `session_id` as an opaque string.

### 3.4 `close`

Params:

```json
{ "session_id": "sess-1" }
```

Result: `{}`

Closing an unknown or already-closed session reports `BUSY` for now; v1 has
no `NOT_OPEN` error code.

### 3.5 `execute`

Params:

```json
{ "session_id": "sess-1", "op": { "op": "spi_transact", "write_b64": "nw==", "read_len": 4 } }
```

`op` is an object whose first key `op` selects the operation. v1 defines:

#### `spi_transact`

```json
{ "op": "spi_transact", "write_b64": "nw==", "read_len": 4 }
```

- `write_b64`: base64-encoded bytes clocked out to the device.
- `read_len`: number of bytes to clock back in.

Result:

```json
{ "data_b64": "70AY/w==" }
```

`data_b64` is base64-encoded received bytes. Its decoded length must equal
`read_len`.

#### `gpio_set`

```json
{ "op": "gpio_set", "pin": "IO0", "level": true }
```

Result: `{}`

A sidecar that did not declare `gpio` in its handshake capabilities must
return `CAPABILITY_NOT_EXPOSED`.

---

## 4. Error codes

|   Code | Name                     | Meaning                                        |
| -----: | ------------------------ | ---------------------------------------------- |
| -32001 | `CAPABILITY_NOT_EXPOSED` | The op/capability was not declared or exposed. |
| -32002 | `BUSY`                   | Adapter/session busy or not open for execute.  |
| -32601 | `METHOD_NOT_FOUND`       | JSON-RPC: unknown method.                      |
| -32602 | `INVALID_PARAMS`         | JSON-RPC: invalid or missing parameters.       |

Host client mapping:

- `-32001` -> `CAPABILITY_NOT_EXPOSED: <message>`
- `-32002` -> `BUSY: <message>`
- Framing and JSON protocol violations produce readable error strings.

---

## 5. Reference implementation

- Host client: `upt-hal` crate, `SidecarClient` and `ChildTransport`.
- Reference sidecar: `upt-devices` crate binary
  `upt_ch34x_sidecar_libusb` / `upt_ch34x_sidecar_dll`.
- Mock sidecar used by tests: `sidecar_mock`.
