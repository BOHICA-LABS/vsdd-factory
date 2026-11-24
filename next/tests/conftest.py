"""Suite-wide fixtures.

Bytecode: the tests import the shipped package in place, and the plugin tree
must stay free of generated files (tools/check.py enforces that), so this
file turns bytecode writing off before any ``vsdd`` import.
"""

import sys

sys.dont_write_bytecode = True
