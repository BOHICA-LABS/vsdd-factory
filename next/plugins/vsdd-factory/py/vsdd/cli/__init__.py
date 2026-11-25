"""The operator tool's command dispatch (packaging.md; topic:command-conventions).

The packages under ``vsdd`` own their commands; this subpackage owns the
dispatcher they all register with. One command is one module named
``cmd_<family>[_<x>].py`` holding ``COMMANDS = (Command(...), ...)``; a unit
adds a command by adding one file and editing none.
"""
