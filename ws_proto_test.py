#!/usr/bin/env python3

# Run pip3 install websockets to install the websockets library
# Run pip3 install protobuf to install the protobuf library
# 'export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python' to use the python implementation of protobuf
# Run 'protoc -I=. --python_out=. protos/sigmiot_data.proto' to generate the python code from the proto file

import asyncio
import websockets

from protos import sigmiot_data_pb2
from dataclasses import dataclass

@dataclass
class SensorValue:
    value_name: str
    value_data: float
    value_unit: str

@dataclass
class SensorData:
    sensor_name: str
    sensor_type: str
    sensor_location: str
    sensor_values: list

def create_sensor_value(value_name, value_data, value_unit):
    sensor_value = SensorValue(value_name, value_data, value_unit)
    return sensor_value

def create_sensor_data(sensor_name, sensor_type, sensor_location, sensor_values):
    sensor_data = SensorData(sensor_name, sensor_type, sensor_location, sensor_values)
    return sensor_data

def append_sensor_data_to_message(message, sensor):
    sensors_data = sigmiot_data_pb2.SensorDataResponse()
    sensors_data.sensor_name = sensor.sensor_name
    sensors_data.sensor_type = sensor.sensor_type
    sensors_data.sensor_location = sensor.sensor_location

    for sensor_value in sensor.sensor_values:
        sensor_values = sigmiot_data_pb2.SensorValue()
        sensor_values.value_name = sensor_value.value_name
        sensor_values.value_data = sensor_value.value_data
        sensor_values.value_unit = sensor_value.value_unit
        sensors_data.sensor_values.extend([sensor_values])

    message.sensors_data_response.extend([sensors_data])
    return message

def append_log_data_to_message(message, log_message, log_timestamp, log_level):
    log_data_resp = sigmiot_data_pb2.LogDataResponse()
    log_data_resp.log_message = log_message
    log_data_resp.log_timestamp = log_timestamp
    log_data_resp.log_level = log_level
    message.log_data_response.extend([log_data_resp])
    return message

def print_message(message):
    serialized_message = message.SerializeToString()

    # Deserialize the message from bytes
    received_message = sigmiot_data_pb2.MessageResponse()
    # Parse a received message from bytes
    received_message.ParseFromString(serialized_message)

    # Access the fields of the received message
    for sensor_data in received_message.sensors_data_response:
        print(sensor_data.sensor_name)
        print(sensor_data.sensor_type)
        print(sensor_data.sensor_location)
        for sensor_value in sensor_data.sensor_values:
            print(sensor_value.value_name)
            print(sensor_value.value_data)
            print(sensor_value.value_unit)

    for log_data in received_message.log_data_response:
        print(log_data.log_message)
        print(log_data.log_timestamp)
        print(log_data.log_level)

async def message_sender(websocket, path):
    temp = 1.0
    hum = 2

    while websocket.open:
        if temp < 30:
            temp += 1.0
        else:
            temp = 5.0

        if hum < 90:
            hum += 1
        else:
            hum = 10

        message = sigmiot_data_pb2.MessageResponse()

        message.status = sigmiot_data_pb2.MessageResponse.Status.OK

        sensor1_value1 = create_sensor_value("temperature", temp, "C")
        sensor1_value2 = create_sensor_value("humidity", hum, "%")
        sensor1_value3 = create_sensor_value("pressure", 3.0, "Pa")
        sensor1_values = [sensor1_value1, sensor1_value2, sensor1_value3]
        sensor1 = create_sensor_data("sensor1", "temperature", "room1", sensor1_values)

        message = append_sensor_data_to_message(message, sensor1)

        sensor2_value1 = create_sensor_value("temperature", temp + 1.0, "C")
        sensor2_value2 = create_sensor_value("humidity", hum + 1, "%")
        sensor2_values = [sensor2_value1, sensor2_value2]
        sensor2 = create_sensor_data("sensor2", "temperature", "room2", sensor2_values)

        message = append_sensor_data_to_message(message, sensor2)

        # Get system time
        time = asyncio.get_event_loop().time()
        message = append_log_data_to_message(message, "log message 1", int(time), "WARN")
        message = append_log_data_to_message(message, "log message 2", int(time), "TRACE")

        print_message(message)

        await websocket.send(message.SerializeToString())
        # Wait for one second before sending the next message
        await asyncio.sleep(1)

async def handle_connection(websocket, path):
    print("Client connected")
    await message_sender(websocket, path)

if __name__ == "__main__":
    print("Starting WebSocket server")
    print("Press Ctrl+C to stop the server")

    # Start the WebSocket server
    asyncio.get_event_loop().run_until_complete(
        websockets.serve(handle_connection, 'localhost', 8080))
    asyncio.get_event_loop().run_forever()
