# vl-lsp - An AutoLISP / VisualLisp Language Server
Language server meant for Helix and other editors to help write VisualLisp / AutoLisp scripts.


# Scope
Cover the core aspects of AutoLISP. Any AutoCAD specific function definitions must be
accessed through the [official help](https://help.autodesk.com)


# Features
None


# Roadmap
The following items will be implemented as work progresses.

- Hover
- Go-to-definition
- Inlay hints
- Completion


# Basic Helix Config
```languages.toml
[[language]]
name = "visual-lisp"
file-types = ["lsp"]
indent = {tab-width = 2, unit = "  "}
auto-format = true
comment-tokens = [";"]
block-comment-tokens = {start = ";|", end = "|;"}
language-servers = ["vl-lsp"]

[language-server.vl-lsp]
logLevel = "DEBUG"
logLocation = "~/.local/state/vl_lsp/log.log"
signaturesLocation = "~/.config/vl_lsp/signatures.lsp"
```


# Contributing
Any contributions are welcome. This will be a labor of love.

