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
`payload`'s type is `{"key": string, "value": int | float | string | boolean, "expires_in": uint32 | null}`  
`int` and `float` are signed 64-bit types.  
`expires_in` is only `null` if no expiry was set when setting the key

### Requests

All requests follow this structure: `{"action": string, "payload": object}`  
`action` can be either `GET`, `SET`, or `DELETE`  
`payload`'s type is `{"key": string | null, "value": int | float | string | boolean, "expires_in": uint32 | null, "username": string | null, "password": string | null}`  
When `action` is `GET`, a `key` is expected, the value will be ignored even if set  
When `action` is `SET`, a `key` and `value` is expected where value can be any supported JSON value other than `null`  
When `action` is `DELETE`, it's the same as `GET`  
When `action` is `AUTH`, `username` and `password` are expected, `AUTH` is required before ANY other operation or they will fail  
`expires_in` is only used in `SET` to set an expiry for a key, and if it's `null`, then the key will never expire
