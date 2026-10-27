# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Bind original MemberRef call primitives in owned generated test carriers."""
import json
import re


def plan(support, harness, fixture, output, bodies, owner, library="-"):
    calls = sorted({(operation, token) for body in bodies for operation, token in re.findall(
        r"__cil_(callvirt|reference_call|memberref_call|methodspec_call)_([0-9A-F]{8})\(", body)})
    aliases = sorted({token for body in bodies for token in re.findall(r"__cil_type_([0-9A-F]{8})", body)})
    if not calls and not aliases:
        return "", ""
    directives = output / "exact-reference-leaves.json"
    support.write_changed(directives, json.dumps({
        "calls": [{"operation": operation, "token": token} for operation, token in calls],
        "aliases": aliases,
    }, indent=2) + "\n")
    result = json.loads(support.run([
        support.DOTNET / "dotnet", harness, fixture, "--reference-call-plan", directives, library,
    ]).stdout)
    helpers = result["support"].replace("typeof(ReferenceCallCases.Ops)", "typeof(" + owner + ")")
    return "\n".join(result["aliases"]) + "\n", helpers + "\n" + "\n".join(result["helpers"]) + "\n"
