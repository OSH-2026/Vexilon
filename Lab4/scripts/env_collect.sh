#!/usr/bin/env bash
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAB4_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT="$LAB4_DIR/results/env_info.txt"

mkdir -p "$LAB4_DIR/results"

{
  echo "# Environment Information"
  echo
  echo "Collected at: $(date -Iseconds 2>/dev/null || date)"
  echo "Hostname: $(hostname 2>/dev/null || true)"
  echo

  echo "## System"
  uname -a 2>/dev/null || true
  echo

  echo "## OS Release"
  if command -v lsb_release >/dev/null 2>&1; then
    lsb_release -a || true
  elif [ -f /etc/os-release ]; then
    cat /etc/os-release
  else
    echo "OS release information not found."
  fi
  echo

  echo "## CPU"
  lscpu 2>/dev/null || wmic cpu get name,numberofcores,numberoflogicalprocessors 2>/dev/null || true
  echo

  echo "## Memory"
  free -h 2>/dev/null || wmic computersystem get TotalPhysicalMemory 2>/dev/null || true
  echo

  echo "## Disk"
  df -h 2>/dev/null || wmic logicaldisk get caption,freespace,size 2>/dev/null || true
  echo

  echo "## Network"
  hostname -I 2>/dev/null || true
  ip addr 2>/dev/null || ipconfig 2>/dev/null || true
  echo

  echo "## Toolchain"
  echo "gcc:"
  gcc --version 2>/dev/null || true
  echo
  echo "g++:"
  g++ --version 2>/dev/null || true
  echo
  echo "cmake:"
  cmake --version 2>/dev/null || true
  echo
  echo "python3:"
  python3 --version 2>/dev/null || python --version 2>/dev/null || true
  echo
  echo "git:"
  git --version 2>/dev/null || true
  echo

  echo "## GPU"
  if command -v nvidia-smi >/dev/null 2>&1; then
    nvidia-smi || true
  else
    echo "nvidia-smi not found. Assume no NVIDIA GPU or driver not installed."
  fi
} > "$OUT"

echo "Environment information written to $OUT"
