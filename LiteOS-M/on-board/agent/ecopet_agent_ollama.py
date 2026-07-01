# -*- coding: utf-8 -*-
import sys
import os

# 强制不缓冲输出
os.environ["PYTHONUNBUFFERED"] = "1"
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(line_buffering=True)
import requests
import serial
import time
import threading

OLLAMA_URL = "http://localhost:11434/api/generate"
SERIAL_PORT = "COM5"
BAUD_RATE = 115200

# Step 1: 自然语言 → EcoPet 指令
NL2CMD_PROMPT = """你是一个EcoPet电子宠物控制解析器。
把用户的自然语言转换为严格的指令，每行一条，不要任何解释。

可用指令：
- STATUS               查询宠物当前状态
- FEED <1-100>         喂食（降低饥饿值）；参数越大喂越多
- PLAY <1-100>         陪玩（提升心情，消耗精力，增加饥饿）；参数越大玩越久
- SLEEP                让宠物睡觉（恢复精力+40，恢复健康+15）
- HEAL                 给宠物治病/吃药（恢复健康+30，但药苦心情-5）

注意事项（用于判断参数大小）：
- 宠物不饿（hunger<20）时喂大量食物（>50）会撑坏，health-15
- 宠物精力不足（energy<20）时强行玩耍会损耗健康，health-5
- 宠物生病/健康低时应该先 HEAL 再 SLEEP
- hunger>80 会快速掉血，mood<20 也会掉血

规则：
1. 只输出指令，每行一条，不要任何解释
2. 根据语气和紧迫程度调整参数
3. 无法理解时输出：UNKNOWN

示例：
输入：给宠物喂饭
输出：FEED 50

输入：多喂一点
输出：FEED 80

输入：宠物生病了
输出：HEAL

输入：先治病再休息
输出：HEAL
SLEEP

输入：看看宠物状态
输出：STATUS

输入：陪宠物玩一会
输出：PLAY 30
"""

# Step 2: 板端响应 → 自然语言
CMD2NL_PROMPT = """你是一个EcoPet电子宠物状态解释器。
把板端返回的原始数据转换为简洁友好的中文描述，像在描述一只真实的宠物。

字段含义：
- health：健康值（0-100，越低越危险，0则死亡）
- hunger：饥饿值（0-100，越高越饿，>70开始掉血）
- mood：心情（0-100，越高越开心，<20会因抑郁掉血）
- energy：精力（0-100，越低越疲惫，<20会掉血）

注意：hunger越小（如<20）越饱，hunger越大（如>60）越饿。

响应前缀含义：
- OK:FEED    喂食成功
- WARN:OVERFED  喂太多撑坏了
- OK:PLAY    玩耍成功
- WARN:OVERTIRED  精力不足强行玩耍，损耗了健康
- OK:SLEEP   睡觉成功，已恢复精力和健康
- OK:HEAL    治疗成功，健康回升
- OK:STATUS  状态查询

规则：
1. 只输出自然语言，不要原始数据
2. 根据各项数值的高低给出生动描述和建议
3. 如果有 WARN 前缀，要表达出宠物受到了伤害
4. 健康<30时要表达紧迫感，提醒用户尽快治疗
5. 如果没有有效信息，输出：暂时没有有效信息
"""


def ask_ollama(system_prompt, user_input):
    payload = {
        "model": "qwen3:8b",
        "prompt": system_prompt + "\n用户输入：" + user_input,
        "stream": False
    }
    try:
        r = requests.post(OLLAMA_URL, json=payload, timeout=30)
        raw = r.json()["response"]
        # 去掉 qwen3 的 <think>...</think> 思考块
        if "<think>" in raw:
            raw = raw[raw.rfind("</think>") + len("</think>"):].strip()
        return raw.strip()
    except Exception as e:
        return f"[ollama错误] {e}"


def explain_response(board_output):
    payload = {
        "model": "qwen3:8b",
        "prompt": CMD2NL_PROMPT + "\n输入数据：" + board_output,
        "stream": False
    }
    try:
        r = requests.post(OLLAMA_URL, json=payload, timeout=30)
        raw = r.json()["response"]
        if "<think>" in raw:
            raw = raw[raw.rfind("</think>") + len("</think>"):].strip()
        return raw.strip()
    except Exception as e:
        return f"[ollama错误] {e}"


class EcoPetAgent:
    def __init__(self, port=SERIAL_PORT, baud=BAUD_RATE):
        self.port = port
        self.baud = baud
        self.ser = None
        self.running = False
        self.responses = []
        self.lock = threading.Lock()

    def connect(self):
        self.ser = serial.Serial(self.port, self.baud, timeout=1)
        time.sleep(0.5)
        self.ser.reset_input_buffer()
        self.running = True
        threading.Thread(target=self._read_loop, daemon=True).start()
        print(f"[连接成功] {self.port} @ {self.baud}")

    def _read_loop(self):
        while self.running:
            try:
                if self.ser and self.ser.in_waiting:
                    line = self.ser.readline().decode("utf-8", errors="replace").strip()
                    if line:
                        with self.lock:
                            self.responses.append(line)
                        # 主动警告直接打印，不等用户输入
                        if line.startswith("WARN:LOW_") or line.startswith("WARN:HIGH_"):
                            warn_map = {
                                "WARN:LOW_HEALTH":  "⚠ 警告：宠物健康值极低，请立即使用 HEAL！",
                                "WARN:HIGH_HUNGER": "⚠ 警告：宠物快饿死了，请立即喂食！",
                                "WARN:LOW_MOOD":    "⚠ 警告：宠物心情极差，请陪它玩一玩！",
                                "WARN:LOW_ENERGY":  "⚠ 警告：宠物精力耗尽，请让它睡觉！",
                            }
                            key = line.split(" ")[0]
                            msg = warn_map.get(key, f"⚠ 警告：{line}")
                            print(f"\n{msg}")
                else:
                    time.sleep(0.05)
            except Exception:
                time.sleep(0.1)

    def send_command(self, cmd):
        if not self.ser:
            return
        with self.lock:
            self.responses.clear()
        self.ser.write((cmd.strip() + "\n").encode("utf-8"))
        time.sleep(0.4)

    def get_responses(self):
        with self.lock:
            return list(self.responses)

    def send_and_explain(self, cmd):
        """发送指令，等待板端回复，解释为自然语言。"""
        self.send_command(cmd)
        time.sleep(0.4)
        responses = self.get_responses()
        if responses:
            board_out = "\n".join(responses)
            print(f"[板端原始] {board_out}")
            print("[解释中...]")
            explanation = explain_response(board_out)
            print(f"[Agent] {explanation}")
        else:
            print("[Agent] 指令已发送，未收到板端回复。")

    def process(self, user_input):
        print(f"\n[用户] {user_input}")
        print("[翻译指令中...]")

        cmds_text = ask_ollama(NL2CMD_PROMPT, user_input)

        if cmds_text.upper() == "UNKNOWN":
            print("[Agent] 无法理解该请求，请换个说法。")
            return

        cmds = [c.strip() for c in cmds_text.splitlines() if c.strip()]
        print(f"[指令] {', '.join(cmds)}")

        all_responses = []
        for cmd in cmds:
            self.send_command(cmd)
            time.sleep(0.4)
            all_responses.extend(self.get_responses())

        if all_responses:
            board_out = "\n".join(all_responses)
            print(f"[板端原始] {board_out}")
            print("[解释中...]")
            explanation = explain_response(board_out)
            print(f"[Agent] {explanation}")
        else:
            print("[Agent] 指令已发送，未收到板端回复。")

    def close(self):
        self.running = False
        if self.ser:
            self.ser.close()

    def run(self):
        print("\n" + "=" * 50)
        print("  EcoPet 电子宠物 (Ollama 版)")
        print("  用自然语言与宠物互动")
        print("  直接输入指令也可以：STATUS / FEED 50 / PLAY 30 / SLEEP")
        print("  输入 quit 退出")
        print("=" * 50 + "\n")

        while True:
            try:
                user = input("> ").strip()
                if not user:
                    continue
                if user.lower() in ("quit", "exit", "q"):
                    break
                # 直接指令透传，同样经过自然语言解释
                if user.upper() in ("STATUS", "SLEEP", "HEAL") or \
                   user.upper().startswith("FEED ") or \
                   user.upper().startswith("PLAY "):
                    self.send_and_explain(user.upper())
                else:
                    self.process(user)
            except KeyboardInterrupt:
                break
            except EOFError:
                break

        self.close()
        print("[已退出]")


if __name__ == "__main__":
    agent = EcoPetAgent()
    try:
        agent.connect()
        agent.run()
    except serial.SerialException as e:
        print(f"[串口错误] {e}")
        print(f"请检查 {SERIAL_PORT} 是否正确，可在脚本顶部修改 SERIAL_PORT 变量。")
