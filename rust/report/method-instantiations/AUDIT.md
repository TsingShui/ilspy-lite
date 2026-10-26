# Generic method instantiation group audit

Five pre-change ILSpy/default-output/original-IL triples are retained under
`audit/manifest.json` with reference sets and command status. All ILSpy/IL
commands succeeded. All five are ILSPY_YES_OURS_NO; no BOTH_NO classification
or denominator exclusions are inferred.

| method | IL observation |
|---|---|
| FSharp Unit.Equals 06000003 | call MethodSpec TypeTestGeneric<Unit> after null branch |
| DiffLib Diff constructor 0600000A | static ToRandomAccess<!0> calls, then independent interface/list operations |
| test RemoveIgnorableLines 0600011F | MethodSpec Where<string>; delegate creation is an independent unsupported operation |
| Decompiler get_HasErrors 0600005D | Any<ValueTuple<...>>, with independently gated delegate and foreign value facts |
| Roslyn CSharpExtensions.Any 06000013 | generic IndexOf<!!0>, then signed comparison |

## Bounded design

Extend exact leaf calls, not the high-level AST. Decode MethodSpec target using
the raw MethodDefOrRef coded index, validate its generic arity/signature and
retain the original MethodSpec token as the executed target. Validate all
supplied arguments in the caller's scope. Substitute declaring-type Var and
method MVar simultaneously: replacement arguments already belong to the caller
and must not be substituted a second time. Reject byref/modifier/vararg shapes
that the existing call model cannot express.

Local reference instance targets use the existing exact direct/callvirt leaves;
never replace a MethodSpec leaf with a bare receiver method name that loses its
generic arguments. Static targets, including external static references, need
no guessed implementation or class layout: the exact original-token operation
retains instantiated owner/method identity, generic type initialization,
constraint/access checks, exceptions and runtime effects. No default empty
method, whole-body proxy or generic interpreter is introduced.

External instance dispatch still needs existing dependency facts and is not
implicitly enabled. Value/constrained receivers, byref returns/params, indirect
calls, generic methods without MethodSpec and unsupported signatures stay gated.

## Runtime evidence and narrower admission

The owned ArgumentOrder GVM gate failed before admission could be counted:
the original generic virtual call on a null Store<string> threw
NullReferenceException with trace=0 before its side-effectful argument helper,
whereas snapshot-based recovered text ran that helper (trace=1) and threw
DivideByZeroException. Reflection invocation reproduced the original result;
JIT minimum-optimization mode also reproduced the discrepancy. Logs are
`behavior-first.log`, `behavior-diagnostic.log` and `behavior-minopts.log`.
The IL still lists the argument helper before callvirt; this evidence exposes
a runtime generic-virtual lookup behavior the current leaf/snapshot model does
not match. No cross-runtime equivalence claim is made.

Generic virtual MethodSpec callvirt is therefore refused with
`generic virtual dispatch needs runtime lookup-order modeling`, including
the original failing owned fixture; it is not removed or hidden. Nongeneric
virtual calls are unchanged. Local generic instance targets must have one exact
declared signature and must be nonvirtual for callvirt admission. Exact direct
call remains a distinct operation. Static, nonvirtual and cross-scoped generic
calls proceed only after independent behavior/mutation gates, including a
side-effectful nonvirtual argument-order counterpart. No GVM gains are counted.

## Validation

Public decompile/scan red before green, with missing/wrong arity and non-generic
target metadata negatives. Owned runtime reconstruction must execute recovered
support bodies, compare value/reference/struct copies, generic/static/virtual
and explicit type argument identity, null/throw and state/initialization order.
Semantic mutants must distinguish wrong generic argument and lost dispatch or
state. Related generic call work can be grouped before the full regression,
static scan and main-session source/IL self-review. No third-party execution.
