"""Gates and jobs: detached job execution and the product environment.

Later gate packages add modules beside this ``__init__``, not text in it. A
module that registers a gate is named ``gate_<name>.py``, which is what the
records' gate registry imports. The job library is ``jobs.py``, its worker
entry is ``job_worker.py`` and the ``vsdd env`` command handlers are ``env.py``.
"""
