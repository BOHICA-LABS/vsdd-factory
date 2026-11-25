---
kind: config
id: component
package: S1
sources: [state.md#2.12]
---
# [[component]]

Optional: a repository with more than one toolchain. Its commands override `[commands]`, the file travels
with the pushed state branch, and a change to it is announced by the configuration fingerprint
([[record:config-fingerprint]]). Keys, types and defaults: [[table:config-keys]].
