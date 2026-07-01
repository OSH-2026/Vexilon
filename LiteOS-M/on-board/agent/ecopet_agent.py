#!/usr/bin/env python3
"""
EcoPet Agent - PC-side natural language interface for the EcoPet demo.
Translates user natural language (Chinese/English) into EcoPet commands,
sends them via serial to STM32F407 running LiteOS-M, and displays responses.

Usage:
    python3 ecopet_agent.py [--port /dev/ttyUSB0] [--baud 115200]
"""

import sys
import os
import time
import threading
import argparse
import json

try:
    import serial
except ImportError:
    print("ERROR: pyserial not installed. Run: pip3 install pyserial")
    sys.exit(1)

try:
    import anthropic
except ImportError:
    print("ERROR: anthropic not installed. Run: pip3 install anthropic")
    sys.exit(1)


SYSTEM_PROMPT = """You are an EcoPet caretaker agent. You manage a virtual electronic pet running on an STM32 microcontroller connected via serial port.

Available commands you can send (one per line, exactly as shown):
- STATUS — query current pet state
- FEED <1-100> — feed the pet (reduces hunger, higher number = more food)
- PLAY <1-100> — play with the pet (improves mood, costs energy)
- SLEEP — put the pet to sleep (restores energy)

Rules:
1. Output ONLY the command(s), one per line, nothing else.
2. Adjust parameter values based on context (e.g., "多喂一点" → FEED 80, "轻轻玩一下" → PLAY 15).
3. If the user asks about status, output STATUS.
4. If the user's request maps to multiple actions, output them on separate lines in logical order.
5. If you cannot map the request to any command, output exactly: UNKNOWN

Examples:
- User: "给宠物喂饭" → FEED 50
- User: "喂多一点" → FEED 80
- User: "和宠物玩一会" → PLAY 30
- User: "让宠物休息" → SLEEP
- User: "看看宠物怎么样了" → STATUS
- User: "先喂饭再让它睡觉" → FEED 50\\nSLEEP
"""


class EcoPetAgent:
    def __init__(self, port, baud=115200):
        self.port = port
        self.baud = baud
        self.ser = None
        self.client = anthropic.Anthropic(base_url='https://www.packyapi.com')
        self.telemetry_lines = []
        self.running = False
        self.reader_thread = None

    def connect(self):
        """Open serial connection to the board."""
        self.ser = serial.Serial(self.port, self.baud, timeout=1)
        time.sleep(0.5)
        self.ser.reset_input_buffer()
        self.running = True
        self.reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self.reader_thread.start()
        print(f"[连接成功] 串口 {self.port} @ {self.baud}")

    def _read_loop(self):
        """Background thread to read serial data."""
        while self.running:
            try:
                if self.ser and self.ser.in_waiting:
                    line = self.ser.readline().decode('utf-8', errors='replace').strip()
                    if line:
                        if line.startswith("TELE:"):
                            self.telemetry_lines.append(line)
                            if len(self.telemetry_lines) > 10:
                                self.telemetry_lines.pop(0)
                        else:
                            print(f"  [板端] {line}")
                else:
                    time.sleep(0.05)
            except Exception:
                time.sleep(0.1)

    def send_command(self, cmd):
        """Send a command string to the board and wait for response."""
        if not self.ser:
            print("[错误] 串口未连接")
            return None

        cmd = cmd.strip()
        self.ser.write((cmd + "\n").encode('utf-8'))
        time.sleep(0.3)

    def translate(self, user_input):
        """Use Claude to translate natural language to EcoPet commands."""
        message = self.client.messages.create(
            model="claude-sonnet-4-6",
            max_tokens=200,
            system=SYSTEM_PROMPT,
            messages=[{"role": "user", "content": user_input}]
        )
        return message.content[0].text.strip()

    def get_last_telemetry(self):
        """Get the most recent telemetry line."""
        if self.telemetry_lines:
            return self.telemetry_lines[-1]
        return None

    def process_input(self, user_input):
        """Main flow: translate → send → display response."""
        print(f"\n[用户] {user_input}")
        print("[翻译中...]")

        commands_text = self.translate(user_input)

        if commands_text == "UNKNOWN":
            print("[Agent] 无法理解该请求，请尝试其他表达方式。")
            return

        commands = [c.strip() for c in commands_text.split('\n') if c.strip()]
        print(f"[Agent → 板端] {', '.join(commands)}")

        for cmd in commands:
            self.send_command(cmd)
            time.sleep(0.5)

    def close(self):
        """Close serial connection."""
        self.running = False
        if self.ser:
            self.ser.close()
            self.ser = None

    def run_interactive(self):
        """Run interactive REPL loop."""
        print("\n" + "=" * 50)
        print("  EcoPet 电子宠物交互系统")
        print("  输入自然语言与宠物互动")
        print("  输入 'quit' 退出, 'tele' 查看最近遥测")
        print("=" * 50 + "\n")

        while True:
            try:
                user_input = input("\n🐾 > ").strip()
                if not user_input:
                    continue
                if user_input.lower() in ('quit', 'exit', 'q'):
                    break
                if user_input.lower() == 'tele':
                    t = self.get_last_telemetry()
                    if t:
                        print(f"  [遥测] {t}")
                    else:
                        print("  [遥测] 暂无数据")
                    continue
                if user_input.upper() in ('STATUS', 'SLEEP') or \
                   user_input.upper().startswith('FEED ') or \
                   user_input.upper().startswith('PLAY '):
                    self.send_command(user_input.upper())
                    continue

                self.process_input(user_input)

            except KeyboardInterrupt:
                break
            except EOFError:
                break

        self.close()
        print("\n[已退出]")


def main():
    parser = argparse.ArgumentParser(description='EcoPet Agent - 电子宠物交互系统')
    parser.add_argument('--port', default='/dev/ttyUSB0',
                        help='Serial port (default: /dev/ttyUSB0)')
    parser.add_argument('--baud', type=int, default=115200,
                        help='Baud rate (default: 115200)')
    args = parser.parse_args()

    api_key = os.environ.get('ANTHROPIC_API_KEY')
    if not api_key:
        print("ERROR: ANTHROPIC_API_KEY environment variable not set.")
        print("Run: export ANTHROPIC_API_KEY='your-key-here'")
        sys.exit(1)

    agent = EcoPetAgent(args.port, args.baud)
    try:
        agent.connect()
        agent.run_interactive()
    except serial.SerialException as e:
        print(f"[串口错误] {e}")
        print(f"请检查串口 {args.port} 是否存在，或尝试其他端口。")
        sys.exit(1)


if __name__ == '__main__':
    main()
