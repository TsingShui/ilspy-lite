# Scoped reference relationship proof

The existing typed IL/CFG and lift pipeline remains the runtime representation.
The relation module proves assignments; it emits no cast operation or object and
never substitutes a method implementation.

## Terms and local proofs

Image-scoped nominal tokens, complete constructed arguments, caller-qualified
type/method variables and vectors form owned terms. Local base and InterfaceImpl
templates substitute only their own Var positions into those arguments.
Known core Object/String identities normalize without namespace guessing.
Hierarchy cycles, arity/position errors and bounded depth/work exhaustion refuse
proof. Generic interface variance requires actual flags, reference arguments
(including explicit reference-type constraints), and the corresponding directional
proof. Value arguments remain invariant; managed-pointer elements are unchanged.

Open-self structural references and declaring Class identities are equivalent
only when the existing scoped registration proved them the same definition.
Field presentation uses that identity, not a generic-base name cast. Other
constructed-base field naming restrictions remain independent.

A newly admitted call argument upcast retains an explicit formal-type operand
so source overload selection cannot silently select the derived overload.
The owned overload mutant removes that operand and chooses 29 instead of 11.

## Exact foreign projection and demand

Dependency binding resolves definition origins using the unchanged explicit
reference policy. The initial implementation projected foreign ancestor templates
only through caller TypeRef representatives. The follow-up in
`report/foreign-hidden-relations/AUDIT.md` also represents a selected foreign
definition without a caller TypeRef, using its image-scoped canonical identity
and bounded dependency facts. Foreign tokens never become caller tokens; missing,
ambiguous or unsupported definitions are not guessed. The IlSpy policy may
select a version substitute, which remains a metadata selection rather than a
runtime-compatibility proof.

Eagerly building every foreign graph made the Roslyn sample take 38.590 seconds
in the Debug probe. Demand-driven retry builds only nominal definitions needed
by a failed typed proof. Aliases use the same whole-image binding registry so
projection does not depend on which method requested an ancestor first.
The context caches that registry by immutable candidate count, version policy
and discovery-completeness stamp; adding references rebuilds it. Image-local
facts/attempts bound retries. A fresh public query with more references can recover
a previously unavailable relationship. Tests check that transition and query order.

The retry is static lowering, not IL execution. No third-party body is executed,
no runtime access/type check is inserted, and unrelated failed operations do not
become successes merely because a dependency graph exists.

## Declared identity spelling and the regression repair

The first full scan found 18 actual lost recoveries, all from spelling formal
reference operands through a class-hierarchy validation that rejected foreign
constructed bases. The qualified declared name is independent of that proof.
A local class may keep reference identity when its base TypeSpec declares a
foreign constructed class; traversal stops without inventing foreign ancestors.
Assignment to that base/its ancestors still keeps complete arguments and the
exact relationship proof. Generic-base field-owner naming is not erased.

The owned NullForeignFormal red/green case reproduces the naming failure.
The corrected carrier executes the recovered private helper, not an original-body
proxy. The final corpus scan restores all 18 and has no recovered-method losses.

## Executable seam

The carrier compiles default recovered operations and verifies declared CLR
types. Static recovered support overloads call recovered support bodies.
The self-cast body retains a generic declaring type, rather than a static generic
method: a real owned failure showed null-this shared generic entry lookup differs
when the declaring context is changed. The corrected carrier preserves that
runtime entry shape, including int/string/object and null receiver cases.
The original failing behavior log is retained.

The owned gate checks identities, reachable field state, generic positions,
null/cast/default flow and exact overload choice. Four mutants fail per
configuration. Negative public tests retain value-variance, erased-position,
missing-constraint and cyclic foreign hierarchy refusals.
