# Void Protocol Specification

The Void protocol currently uses TCP. Support for UDP may be added in the future.

## Serialization Format

MessagePack is used for serialization, due to fast implementations being available for many languages, and its small size.

| Field Size (bytes) | Format                 | Description                                                            |
| ------------------ | ---------------------- | ---------------------------------------------------------------------- |
| 4                  | uint32 (little-endian) | Size of the (potentially compressed) MessagePack formatted message.    |
| 1                  | uint8                  | Compression mode. If 0, the next 4 bytes will be part of the message.  |
| 4                  | uint32 (little-endian) | Size of the uncompressed data. Only sent if compression mode is not 0. |
| ...                | \[uint8\]              | The (potentially compressed) MessagePack formatted message.            |

### Compression

At this time, compression is only used in requests.

| Bit | Mode    |
| --- | ------- |
| 1   | LZ4     |
| 2   | Zstd    |
| 3   | Snappy  |
| 4   | Brotli  |
| 5   | DEFLATE |
| 6   | zlib    |
| 7   | gzip    |
| 8   | LZW     |

## Custom Types

- `PrimitiveValue`: `int64 | uint64 | float64 | string | boolean`
- `InsertTableValue`: `{ "value": PrimitiveValue, "lifetime": uint64 | null }`

  - `lifetime` is seconds from the current time. If null, the data will never expire

- `TableValue`: `{ "value": PrimitiveValue, "expiry": uint64 | null }`

  - `expiry` is seconds from the Unix epoch. If null, the data will never expire

- `InsertTable`: `{ (...keys): InsertTableValue }`
- `Table`: `{ (...keys): TableValue }`

## Responses

All responses follow this structure: `{ "status": string, (...data) }`

### Statuses

- `Success`: Operation succeeded with zero errors
- `Too many connections`: Server reached its connection limit
- `Malformed request`: Request was built improperly
- `Server error`: Error occured on the server (this is a bug!)

- `Request too large`: Request does not fit in the server's configured message size
- `Response too large`: Requested data could not fit in the server's configured message size

- `Unauthorized`: `AUTH` is required for this operation OR `AUTH` failed
- `Permission denied`: Client doesn't have permission to perform the action

- `Already exists`: Tried to create table or key which already exists
- `No such table`: Operation was attempted on a non-existent table
- `No such key`: Operation was attempted on a non-existent key
- `Key expired`: Operation was attempted on an expired key

## Requests

All requests follow this structure: `{ "action": string, (...data) }`

### Actions

#### Unauthorized

| Action(s) | Request Data                             | Server Data (on success) |
| --------- | ---------------------------------------- | ------------------------ |
| `PING`    | ...                                      | ...                      |
| `AUTH`    | `"username": string, "password": string` | ...                      |

#### Privileged

| Action(s)      | Request Data                                               | Server Data (on success) |
| -------------- | ---------------------------------------------------------- | ------------------------ |
| `LIST TABLE`   | ...                                                        | `"tables": [string]`     |
| `INSERT TABLE` | `"table": string, "contents": InsertTable \| null`         | ...                      |
| `GET TABLE`    | `"table": string`                                          | `Table`                  |
| `DELETE TABLE` | `"table": string`                                          | ...                      |
|                |                                                            |                          |
| `LIST`         | `"table": string`                                          | `"keys": [string]`       |
| `INSERT`       | `"table": string, "key": string, "item": InsertTableValue` | ...                      |
| `GET`          | `"table": string, "key": string`                           | `TableValue`             |
| `DELETE`       | `"table": string, "key": string`                           | ...                      |
