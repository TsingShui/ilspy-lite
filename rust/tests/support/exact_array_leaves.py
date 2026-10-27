# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Bind individual array operations, never selected bodies, in owned carriers."""
import json
import re


def plan(support, harness, fixture, output, bodies):
    directives = []
    for token, body in bodies:
        allocations = sorted(set(re.findall(r"__cil_newarr_([0-9A-F]{8})", body)))
        elements = sorted(set(re.findall(r"__cil_(ldelem|ldelema|stelem)_([0-9A-F]{8})", body)))
        if allocations or elements:
            directives.append({"token": int(token, 16), "aliases": [],
                "allocations": [int(value, 16) for value in allocations],
                "elements": [{"op": op, "token": int(value, 16)} for op, value in elements]})
    if not directives:
        return "", [body for _, body in bodies]
    path = output / "exact-array-leaves.json"
    support.write_changed(path, json.dumps(directives, indent=2))
    rows = json.loads(support.run([support.DOTNET / "dotnet", harness, fixture,
                                  "--typed-array-plan", path]).stdout)
    if len(rows) != len(directives):
        raise ValueError("Array leaves require one closed selection per body")
    by_token = {row["token"]: row for row in rows}
    classes, rebound = [], []
    for token, body in bodies:
        numeric = int(token, 16)
        if numeric in by_token:
            row = by_token[numeric]
            owner = "__ArrayLeaves_" + token[2:]
            helpers = "\n".join(row["helpers"]).replace("private ", "public ")
            classes.append("internal static class " + owner + " {\n" + helpers + "\n}")
            body = re.sub(r"\b(__cil_(?:newarr|ldelem|stelem|ldelema)_[0-9A-F]{8})\(",
                          lambda match: owner + "." + match[1] + "(", body)
        rebound.append(body)
    return "\n".join(classes) + "\n", rebound
