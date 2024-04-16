# Void Protocol Specification

All connections are made over TCP.

# Responses

Responses follow this structure: `[error_byte, msg_bytes, data]`.

- `error_byte` can be 0 or 1. If it's 0, there is no error; if it's 1, there is an error.

- `msg_bytes` can be 0 or a value greater than 0. If it's 0, there is no message; otherwise, there is a message. In the case of a message, it is sent in ASCII format and terminates with a null byte (0).

- `data` can be 0 or any other value. If it's 0, there is no additional data; otherwise, there is one or more bytes of data.
