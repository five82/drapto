# AGENTS.md

CLAUDE.md and GEMINI.md are symlinks to this file.

## TL;DR

- Do not run `git commit` or `git push` unless explicitly instructed
- Run `./check-ci.sh` before handing work back
- Use Context7 MCP for library/API docs without being asked

## Project

Drapto is an FFmpeg wrapper for AV1 encoding. Single-developer hobby project.

## Related Repos

| Repo | Path | Role |
|------|------|------|
| drapto | `~/projects/drapto/` | FFmpeg encoding wrapper (this repo) |
| spindle | `~/projects/spindle/` | Orchestrator that uses Drapto as a library |
| flyer | `~/projects/flyer/` | Read-only TUI for Spindle |

GitHub: [drapto](https://github.com/five82/drapto) | [spindle](https://github.com/five82/spindle) | [flyer](https://github.com/five82/flyer)

## Commands

```bash
go build -o drapto ./cmd/drapto
go test ./...
golangci-lint run
./check-ci.sh
```

## Principles

1. Keep it simple - small hobby project
2. Prefer unit tests over actual encodes
3. Use 120s+ timeout when running drapto
