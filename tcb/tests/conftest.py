"""Põe tcb/ no sys.path para os testes importarem `tcb` sem instalar nada."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
