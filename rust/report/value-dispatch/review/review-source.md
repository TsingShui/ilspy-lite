# Constrained dispatch and reference control flow: source/IL self-review

Main-session bounded static self-review, not independent review, third-party
execution or a whole-corpus equivalence proof. Ten production default readable
outputs were compared with ILSpy and original IL (`manifest.json`). Original IL
is authoritative. Ten CRUDE-BUT-EQUIVALENT, zero changed-semantic SUSPECT in this
sample. All ten reference invocations succeeded without unknown-result warnings.

| triple | changed-semantic checks |
|---|---|
| 0001-06000007 AlignedDiffChange.GetHashCode | two separate T field snapshots/local locations feed the exact constrained T/Object.GetHashCode pairs; unchecked 397 multiplication/xor and the later enum box/ordinary call stay ordered. |
| 0003-06000070 WritePreProcessorDirective | first output.Write precedes enum slot acquisition; constrained enum ToString then String.ToLowerInvariant and output calls retain all tokens, null/empty branching and char narrowing. |
| 0008-06000046 DisposableEnumeratorAdapter.get_Current | !0 result and !1 receiver remain distinct. Object enumerator is tested first; only its null path acquires the struct field location and constrained interface dispatch. No copied/boxed receiver replaces that location. |
| 0003-06000072 WritePrimitiveType | hash/string-test CFG remains explicit; the null conditional GetDefinition path merges only null with exact ITypeDefinition. Typed edge scratch values retain null, no getter executes on the null path, and output paths/argument order remain unchanged. |
| 0895-0600012E GetEffectiveRefKindAndMarkMatchedInParameter | original ref ArrayBuilder slot crosses both coalescing edges by ref-bound temporaries, not contents copies. Factory executes only for null; stind.ref writes the same acquired slot before a fresh slot read and set_Item. RefKind byte normalization is preserved. |
| 0000-060004E4 QueueContinuationWithTrampoline | ldobj copies original AsyncActivation<T> to l0; byref local l1 aliases that copy, not original this. Nested reads, result/value arguments, constructor and queue call keep their original order and references. |
| 1041-0600006B CPos.ToString | original-this location feeds exact constrained CPos/Object.ToString before LuaValue implicit conversion. Neither a pre-box nor direct Object call is substituted. |
| 1041-060000C1 Exts.Clamp | each acquired by-value val argument slot feeds constrained IComparable<T>.CompareTo with min/max snapshots; second call occurs only after first comparison permits it. Return values remain whole T copies. |
| 1042-06000438 GeneralProperties.get_Stance | null autotarget returns null without dereference. Non-null path makes the original direct stance getter call, stores the enum then takes its local address for constrained Object.ToString. |
| 0627-0600003F AssemblyName.set_CultureInfo | this reference is carried separately; value.Name runs only for non-null input. Null/string result uses destination-typed edge temporaries before one original field store, without object-to-string erasure. |

## Owned behavioral scope

`constrained-roundtrip.py` compiles default recovered bodies and uses actual typed
shared-cell pointers. Dynamic operation leaves contain only original constrained
and callvirt operations; they do not call the selected Ops/Host methods. Recovered
Ledger.Argument/Replace support bodies preserve argument effects. Type-owner and
method-owner specializations execute separately. Primitive, nullable, reference,
overridden struct and non-overridden struct receiver paths are covered, including
null reference receivers, derived dispatch, throwing calls and caller-local copies.

The current gate runs 720 comparisons / 40 closed selections per configuration.
Five mutants reject copied receiver locations, lost argument traces, removed
reference-slot snapshots, wrong joined locations and copied array storage. The
array carry fixture is authored original IL retaining an acquired uint element
pointer across both branch edges; both successful branches, null arrays and
out-of-range indices execute. Original input storage and captured references are
compared after normal and exceptional returns.

Legacy promotions execute before frozen negative rows are removed: managed
location joins and Debug byref locals, generic null-conditional allocation-fixture
method, and virtual fixture constrained value ToString (bound to its already
recovered value receiver body). The virtual oracle initially required reference
identity between two independently produced strings; equal contents and separate
repeat-call freshness checks correctly distinguish those observations. Compiler
ref-escape errors in the initial CFG carrier were fixed by scoped scratch refs,
not by suppressing C# diagnostics or weakening runtime checks.

The complete 24-driver milestone manifest passes. GVM's previous counterexample
and stable refusal remain. Pinned/modified/unmanaged locals, argument-slot
rebinding, byref result/return escapes and malformed prefix entry retain gates.
CVec.Minus remains Partial at a missing external constructor; its reference
unknown-result diagnostic is inconclusive and never changes corpus scope.
