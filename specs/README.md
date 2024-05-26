# Void Specification

Void is written in Rust with safety in mind while focusing on being fault tolerant and scalable.

## Core Design Decisions

- Multithreaded Architecture
- Memory Safety Priority
- Atomic Write Operations
- Durability Through Persistence
- Isolation for Transactions

## Protocol Specification

The client initiates a connection by establishing a TCP connection with the server and then authenticating.
Upon successful connection, both the client and server can start exchanging data.
We use JSON for request/response to make it an easier developer experience.

### Custom Types

- `PrimitiveValue`: `int64 | uint64 | float64 | string | boolean`
- `ResponsePayload`: `{ "key": string, "value": PrimitiveValue, "expires_in": uint64 | null }`
  - `expires_in` is only `null` if no expiry was set when setting the key

### Responses

All responses follow this structure: `{ "error": boolean, "message": string | null, "payload": ResponsePayload | null }`

- `error` is set to true when there's an error
- `message` is set to inform the client

### Requests

All requests follow this structure: `{ "action": string, ...(data) }`  
`action` can be either `AUTH`, `GET`, `DELETE`, or `SET`

#### `data`

- When `action` is `AUTH`, `"username": string, "password": string` is expected

  - `AUTH` is required before _any_ other operation or they will fail

- When `action` is `GET` or `DELETE`, `"key": string` is expected

- When `action` is `SET`, `"key": string, "value": PrimitiveValue, "expires_in": uint64 | null` is expected

  - If `expires_in` is null, the value will never expire
