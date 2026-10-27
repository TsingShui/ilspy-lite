
## Follow-up group: runtime casts to referenced reference types

The cast bucket (7,691 first blockers) was refused because `cast_class` required a
*local* class definition. ILSpy instead resolves the cast's type through
`MetadataModule.ResolveType` and keeps the operation; correctness only needs the
exact type identity and whether the type is a reference type, because `castclass`
on a value type addresses a boxed value.

`DependencyContext::external_type_kind` now reads the selected definition's flags
and `ValueType`/`Enum` base; `cast_class` uses it for TypeRef targets after the
existing name-shape check, and a new priming pass fills the cache per body.
Admission adds no assignability proof, boxing or allocation, and unavailable
definitions keep an explicit refusal.

Owned evidence: two Ops casts were red before the fact and green after, the same
methods stay Partial without a library, and a unit test checks the fact source
(reference kinds versus the owned `Bits` struct). 471 tests pass; the relation
driver executes 187 comparisons / 44 selections / 8 mutants per configuration.

Local static measurement: NRefactory.CSharp.Refactoring +473, System +333,
System.Data.Common +237, mscorlib 4.5 +24, OpenRA.Mods.Common +28; CoreLib
unchanged. Full scan pending.
