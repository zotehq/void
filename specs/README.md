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

### Responses

All responses follow this structure: `{"error": boolean, "message": string, "payload": null | object}`  
Error is set to true when there's an error (duh!)  
Message is set to inform the client (duh!)  
Payload is of this type: `{"key": string, "value": string, "type": string, "expires_in": int32 | null}`  
`expires_in` is only `null` if no expiry was set when setting the key

### Requests

All requests follow this structure: `{"action": string, "key": string | null, "value": string | null, "type": string | null, "expires_in": int32 | null}`  
`action` can be either `GET`, `SET`, or `DELETE`  
When `action` is `GET`, a `key` is expected, the value will be ignored even if set  
When `action` is `SET`, a `key` and `value` is expected where value can be any supported JSON value other than `null`  
When `action` is `DELETE`, it's the same as `GET`  
`expires_in` is only used in `SET` to set an expiry for a key, and if it's `null`, then the key will never expire
