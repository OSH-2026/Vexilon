#!/usr/bin/env python3
"""
EcoPet Serial MCP Tool - A Model Context Protocol (MCP) server that provides
serial communication tools for IronClaw to interact with the EcoPet board.

This runs as an MCP stdio server. IronClaw spawns it as a subprocess and
communicates via JSON-RPC over stdin/stdout.
"""

import sys
import json
import time
import threading

try:
    import serial
except ImportError:
    sys.stderr.write("ERROR: pyserial not installed\n")
    sys.exit(1)

PORT = "/dev/ttyUSB0"
BAUD = 115200

ser = None
rx_buffer = []
rx_lock = threading.Lock()
running = True


def serial_reader():
    """Background thread reading serial lines."""
    global running
    while running:
        try:
            if ser and ser.in_waiting:
                line = ser.readline().decode('utf-8', errors='replace').strip()
                if line:
                    with rx_lock:
                        rx_buffer.append(line)
                        if len(rx_buffer) > 50:
                            rx_buffer.pop(0)
            else:
                time.sleep(0.05)
        except Exception:
            time.sleep(0.1)


def open_serial():
    """Open the serial port."""
    global ser
    import os
    port = os.environ.get("ECOPET_SERIAL_PORT", PORT)
    baud = int(os.environ.get("ECOPET_BAUD", BAUD))
    ser = serial.Serial(port, baud, timeout=1)
    time.sleep(0.5)
    ser.reset_input_buffer()
    t = threading.Thread(target=serial_reader, daemon=True)
    t.start()


def send_command(command):
    """Send command to board and collect response."""
    if not ser:
        return {"error": "Serial port not connected"}

    with rx_lock:
        rx_buffer.clear()

    ser.write((command.strip() + "\n").encode('utf-8'))
    time.sleep(0.5)

    with rx_lock:
        responses = [l for l in rx_buffer if l.startswith("OK:") or l.startswith("ERR:")]

    if responses:
        return {"response": responses[-1]}

    time.sleep(0.5)
    with rx_lock:
        responses = [l for l in rx_buffer if l.startswith("OK:") or l.startswith("ERR:")]

    if responses:
        return {"response": responses[-1]}
    return {"response": "NO_RESPONSE (board may not have replied)"}


def read_telemetry():
    """Get recent telemetry lines."""
    with rx_lock:
        tele = [l for l in rx_buffer if l.startswith("TELE:")]
    if tele:
        return {"telemetry": tele[-1]}
    return {"telemetry": "NO_DATA"}


# MCP Protocol handlers

def handle_initialize(params):
    return {
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "ecopet-serial", "version": "0.1.0"}
    }


def handle_tools_list(params):
    return {
        "tools": [
            {
                "name": "send_command",
                "description": "Send a command to the EcoPet board via serial. Valid commands: STATUS, FEED <1-100>, PLAY <1-100>, SLEEP",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to send (e.g., 'FEED 50', 'STATUS', 'PLAY 30', 'SLEEP')"
                        }
                    },
                    "required": ["command"]
                }
            },
            {
                "name": "read_telemetry",
                "description": "Read the most recent telemetry data from the EcoPet board",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    }


def handle_tools_call(params):
    name = params.get("name")
    args = params.get("arguments", {})

    if name == "send_command":
        result = send_command(args.get("command", ""))
    elif name == "read_telemetry":
        result = read_telemetry()
    else:
        result = {"error": f"Unknown tool: {name}"}

    return {
        "content": [{"type": "text", "text": json.dumps(result)}]
    }


def process_request(req):
    """Route JSON-RPC request to handler."""
    method = req.get("method", "")
    params = req.get("params", {})

    if method == "initialize":
        return handle_initialize(params)
    elif method == "notifications/initialized":
        return None
    elif method == "tools/list":
        return handle_tools_list(params)
    elif method == "tools/call":
        return handle_tools_call(params)
    elif method == "ping":
        return {}
    else:
        return {"error": {"code": -32601, "message": f"Method not found: {method}"}}


def main():
    open_serial()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue

        result = process_request(req)
        if result is None:
            continue

        response = {
            "jsonrpc": "2.0",
            "id": req.get("id"),
            "result": result
        }
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
