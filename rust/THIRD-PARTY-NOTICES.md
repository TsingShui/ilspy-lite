# Third-party notices

Direct dependencies are pinned `cildec 0.1.0`, `sha1 0.10.7` and
`serde_json 1.0.154`. `serde_json` parses bounded local dependency manifests;
its lockfile closure includes itoa, memchr, serde, serde_core, serde_derive,
proc-macro2, quote, syn, unicode-ident and zmij. The full MIT notices for that
closure, plus unicode-ident's required Unicode-3.0 notice, are retained in
[licenses/dependency-licenses.txt](licenses/dependency-licenses.txt).
`cildec` supplies the PE/CLI, metadata/signature and IL decoder; its default `std`
feature introduces no other normal dependencies. `sha1` implements the legacy
ECMA-335 public-key-token identity calculation, not signature authentication or
new cryptographic security. All versions are pinned by Cargo.lock.

The SHA-1 closure also contains block-buffer, cfg-if, cpufeatures, crypto-common,
digest, generic-array, libc (on applicable targets), typenum and version_check.
Their exact versions, original copyright notices and full MIT license texts are
retained in [licenses/dependency-licenses.txt](licenses/dependency-licenses.txt).
Where dual-licensed, this distribution selects the MIT license. Include this
notice and that license file with redistributed binaries.

## cildec 0.1.0

```text
MIT License

Copyright (c) 2026 binsnake

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

```
