# Void Protocol Specification

The client initiates a connection by establishing a raw TCP or WebSocket connection with the server and then authenticating.
Upon successful connection, both the client and server can start exchanging data.

Serialization format depends on the connection protocol. For raw TCP, MessagePack is used, and for WebSocket, JSON is used.

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

- `OK`: Operation succeeded with zero errors
- `Too many connections`: Server reached its connection limit
- `Malformed request`: Request was built improperly
- `Server error`: Error occured on the server (this is a bug!)

- `Unauthorized`: `AUTH` is required for this operation
- `Forbidden`: Client doesn't have permission to perform the action
- `Invalid credentials`: `AUTH` was attempted with invalid credentials

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
