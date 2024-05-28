# Void Protocol Specification

The client initiates a connection by establishing a raw TCP or WebSocket connection with the server and then authenticating.
Upon successful connection, both the client and server can start exchanging data.
JSON is used for messaging to keep the protocol simple.

## Custom Types

- `PrimitiveValue`: `int64 | uint64 | float64 | string | boolean`
- `MapData`: `{ "key": string, "value": PrimitiveValue, "expires_in": uint64 | null }`

  - `expires_in` is in seconds. If null, the data will never expire

- `PingPongPayload`: `string`

  - `PingPongPayload` is a `string`, but with two important limitations
  - It must be valid base64, and the underlying data must NOT be over 125 bytes
  - Size limit matches the WebSocket ping/pong message size limit for ease of implementation

## Responses

All responses follow this structure: `{ "status": string | null, "payload": MapData | PingPongPayload | null }`

### `status`

- `OK`: Operation succeeded with zero errors
- `Too many connections`: Server reached its connection limit
- `Malformed request`: Request was built improperly

- `Pong!`: Pong response to a ping

  - `payload` will be equal to the one sent by the client

- `Authentication required`: `AUTH` is required for this operation
- `Invalid credentials`: `AUTH` was attempted with invalid credentials
- `Already authenticated`: `AUTH` was attempted after earlier successful AUTH

- `Key expired`: `GET` was attempted on an expired key
- `No such key`: `GET` was attempted on a non-existent key

## Requests

All requests follow this structure: `{ "action": string, ...(data) }`  
`action` can be either `PING`, `AUTH`, `GET`, `DELETE`, or `SET`

### `data`

- When `action` is `PING`, `"payload": PingPongPayload` is expected

  - The server will send back a `Pong!` status with the same `payload`

- When `action` is `AUTH`, `"username": string, "password": string` is expected

  - `AUTH` is required before _any_ other operation or they will fail

- When `action` is `GET` or `DELETE`, `"key": string` is expected

- When `action` is `SET`, inline `MapData` is expected
