# TCP

All connections are done over TCP

# Responses

Responses look something like this: `[error_byte, msg_bytes, data]`  
`error_byte` can be a 0 or a 1, if it is 0, then there is no error, and if it's a 1, then there is an error

`msg_bytes` can be 0 or can be a byte higher than 0, if it's a 0, then there is no message, else there is a message  
In the case that there is a message, it is sent over ASCII, and it will only end if you reach a 0.

`data` can be 0 or anything else, if it is 0, then there is no data, if it is not a 0, then there is one or more bytes
