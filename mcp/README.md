# Gridvana MCP Service

This crate provides the reusable Gridvana edit-session engine, MCP JSON-RPC
protocol handler, and the standalone `gridvana-mcp-service` binary.

## Build

```sh
cargo build -p gridvana_mcp --bin gridvana-mcp-service
```

## Stdio

The service defaults to read-only. Read-only mode exposes resources but no
write tools:

```sh
gridvana-mcp-service --stdio --project ./demo.gvn --readonly
```

Enable writes only for a trusted local Agent. A committed session is written
back to the project file:

```sh
gridvana-mcp-service --stdio --project ./demo.gvn --write
```

## Streamable HTTP

HTTP listeners are restricted to loopback addresses:

```sh
gridvana-mcp-service \
  --http 127.0.0.1:17321 \
  --project ./demo.gvn \
  --readonly
```

The MCP endpoint is `http://127.0.0.1:17321/mcp`. A lightweight health endpoint
is available at `/health`.

## Resources

- `gridvana://project/summary`
- `gridvana://selection/current`
- `gridvana://schema/edit-op`
- `gridvana://frame/active.png`

The project, selection, and EditOp V2 formats use stable numeric
`layer_id`/`frame_id`/`cel_id` values. Pixel edits use `set_cel_pixels` or
`erase_cel_pixels`; frame and layer array positions are display order only and
are not accepted as persistent edit targets. Preview/apply responses include
the affected layer, frame, and cel IDs.

When the editor has not created a canvas yet, the project summary reports a
zero width and height. Start an edit session and apply `resize_canvas` with
`canvas_width` and `canvas_height` before sending pixel edits. The service does
not expose `replace_project`.

## Tools

- `gridvana_start_edit_session`
- `gridvana_preview_edit_ops`
- `gridvana_apply_edit_ops`
- `gridvana_commit_session`
- `gridvana_rollback_session`

Only one write session is active at a time. Every session is bound to a
`base_revision`; project changes outside the session cause subsequent writes
to return a revision conflict. Preview and apply operations use a working copy.
Only commit replaces the current project.

The embedded Gridvana server enables writes. Codex or Claude handles any user
approval in its own terminal before invoking the MCP tools.
