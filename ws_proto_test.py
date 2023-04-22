#!/usr/bin/env python3

import asyncio
import websockets

import sigmiot_data_pb2
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
    sensors_data = sigmiot_data_pb2.SensorData()
    sensors_data.sensor_name = sensor.sensor_name
    sensors_data.sensor_type = sensor.sensor_type
    sensors_data.sensor_location = sensor.sensor_location

    for sensor_value in sensor.sensor_values:
        sensor_values = sigmiot_data_pb2.SensorValue()
        sensor_values.value_name = sensor_value.value_name
        sensor_values.value_data = sensor_value.value_data
        sensor_values.value_unit = sensor_value.value_unit
        sensors_data.sensor_values.extend([sensor_values])

    message.sensor_data_response.sensors_data.extend([sensors_data])
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
    # Parse a received message from bytes
    received_message = sigmiot_data_pb2.MessageResponse()
    received_message.ParseFromString(serialized_message)

    # Access the fields of the received message
    for sensor_data in received_message.sensor_data_response.sensors_data:
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
    message = sigmiot_data_pb2.MessageResponse()

    sensor1_value1 = create_sensor_value("temperature", 1.0, "C")
    sensor1_value2 = create_sensor_value("humidity", 2.0, "%")
    sensor1_values = [sensor1_value1, sensor1_value2]
    sensor1 = create_sensor_data("sensor1", "temperature", "room1", sensor1_values)

    message = append_sensor_data_to_message(message, sensor1)

    sensor2_value1 = create_sensor_value("temperature", 3.0, "C")
    sensor2_value2 = create_sensor_value("humidity", 4.0, "%")
    sensor2_values = [sensor2_value1, sensor2_value2]
    sensor2 = create_sensor_data("sensor2", "temperature", "room2", sensor2_values)

    message = append_sensor_data_to_message(message, sensor2)

    message = append_log_data_to_message(message, "log message 1", 123456789, "WARN")
    message = append_log_data_to_message(message, "log message 2", 123456789, "TRACE")

    print_message(message)

    while websocket.open:
        await websocket.send(message.SerializeToString())
        # Wait for one second before sending the next message
        await asyncio.sleep(1)

async def handle_connection(websocket, path):
    print("Client connected")
    await message_sender(websocket, path)

if __name__ == "__main__":
    message = sigmiot_data_pb2.MessageResponse()

    print_message(message)

    # Start the WebSocket server
    asyncio.get_event_loop().run_until_complete(
        websockets.serve(handle_connection, 'localhost', 8080))
    asyncio.get_event_loop().run_forever()
