# Chapter One

Sibling-of-src include, still inside the book root (must succeed -- this is
how the official Rust Book's `{{#include ../listings/...}}` pattern works):

```text
{{#include ../sibling.txt}}
```

Relative traversal outside the book root (must be rejected):

```text
{{#include ../../secret.txt}}
```

Absolute path:

```text
{{#include /etc/passwd}}
```
