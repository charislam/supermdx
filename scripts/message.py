#!/usr/bin/env python3
import json
import sys

def write_message(message):
    json_content = json.dumps(message)
    content_length = len(json_content)
    sys.stdout.buffer.write(f"Content-Length: {content_length}\r\n\r\n".encode('ascii'))
    sys.stdout.buffer.write(json_content.encode('utf-8'))
    sys.stdout.buffer.flush()

# Example usage
write_message({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {"capabilities":{}}
})
