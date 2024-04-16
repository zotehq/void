# TCP

All connections are done over TCP

# Responses

Responses look something like this: `[error_byte, msg_bytes, data]`  
`error_byte` can be a 0 or a 1, if it is 0, then there is no error, and if it's a 1, then there is an error  
`msg_bytes` can be 0 or can start with 255, if it's a 0, then there is no message, else there is a message

In the case that there is a message, it is sent over ASCII, and it will only end if you reach a 255. If there is a 255 byte but the message is not over, then it will be terminated by a 254, kinda like a backslash, and if there's a 254, then it will also be terminated by a 254, kinda like `\\`

`data` can be 0 or anything else, if it is 0, then there is no data, if it is not a 0, then there is one or more bytes
