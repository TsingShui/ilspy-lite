# Defect: the allocation-owner check rejects every sealed class

Status: open, queued for the next worker dispatch (highest value per line changed).

## Evidence

`src/metadata.rs:3004-3007`:

```rust
let definition = self.md.tables().type_def(owner.rid())?;
if definition.flags & (0x20 | 0x80 | 0x140) != 0 {
    return Err("allocation owner must be a concrete class".into());
}
```

`0x140` is `TypeAttributes.Sealed (0x100) | 0x40`, and `0x40` is not a `TypeDef` flag.
`0x20` is interface and `0x80` is abstract, both correct rejections; **sealed is the
opposite of "not concrete"** - a sealed class is instantiable, and most small classes
(`<>c__DisplayClass*`, syntax nodes, options objects) are sealed.

Measured effect: `allocation owner must be a concrete class` is the first blocker for
**28,685 bodies**, and sampled `newobj` targets are ordinary sealed local classes:

- `Microsoft.CodeAnalysis.VisualBasic.Syntax.InternalSyntax.MethodBlockSyntax::.ctor`
- `Newtonsoft.Json.Tests.Linq.LinqToJsonTest/<>c__DisplayClass65_0::.ctor`
- `IKVM.Reflection.Emit.AssemblyBuilder/TypeForwarder::.ctor`

## Fix

Reject only `interface | abstract` at this site. A C# static class is `abstract | sealed`,
so keeping `0x80` still refuses it; no other bit is needed. Do not special-case sealed.

## Secondary: diagnosis quality

The same string is also the catch-all for shapes that fail later (generic arity, generic
declaring context, ancestry, constructor signature, parameter types), so it preempts
precise reasons. Split those into their own stable reasons while touching this code.

## Expected effect

Up to 28,685 bodies become eligible, of which those whose constructor parameters and owner
facts are already admitted should decompile immediately; the rest should move to a precise
reason. Measure both the recovered delta and the reason-mix change.

## Heterogeneity caveat (measured)

The bucket is not uniform, so 28,685 is an upper bound for this fix:

- Random sample of 30 methods: 44 `newobj` sites, 34 with a local `TypeDef` ctor
  (`0x06`), 7 external `MemberRef`, 3 generic `MemberRef`. A `TypeDef` ctor can still be
  a *generic nested* type whose instantiation needs constructed-owner handling.
- Counterexample checked directly (`FSharp.Core` `0x06000153`): the site is
  `newobj 0x0A000091 class $Prim-types/op_Implicit@2891<!0, !1>::.ctor`, i.e. an open
  generic instantiation, which is a different limitation from the sealed bit.

So fixing the mask is necessary and cheap, but the slice must also **split the reason
string** (as required above) so the remaining causes are attributable: sealed-local
classes should recover, while generic instantiation and external owners should surface
as their own stable reasons rather than hiding behind this one.
