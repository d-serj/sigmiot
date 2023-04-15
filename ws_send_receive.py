from websocket import create_connection
import json

ws = create_connection("ws://ESP-IP-ADDRESS/ws")

print("Sending 'Request'")
ws.send("Request")
print("receiving...")
result = ws.recv()
print("Received '%s'" % result)

ws.close()
