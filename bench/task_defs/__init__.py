"""The self-validation tasks, one module per task: the module name is the task name and `TASK` is its definition.

`T` keeps the order of `NAMES`, which is the order `make_tasks.py` writes the tasks in."""
from importlib import import_module

NAMES = [
    "py_paginate",
    "py_stack_peek",
    "py_parse_kv",
    "py_intervals",
    "py_ratelimit",
    "py_flatten",
    "ts_chunk",
    "ts_emitter",
    "py_lru",
    "py_wordcount",
    "ts_retry",
    "ts_matrix",
    "py_slug",
    "py_account",
    "ts_group",
    "ts_queue",
    "py_httpresp",
    "py_guard",
    "py_money",
    "ts_listsig",
]

T = {name: import_module(f"{__name__}.{name}").TASK for name in NAMES}
