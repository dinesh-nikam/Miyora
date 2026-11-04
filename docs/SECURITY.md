# Miyora 3D Studio — Security Notes

Scope: `crates/studio` and `apps/desktop/src/studio`. Project-wide policy is in
the root `SECURITY.md`.

## Secrets

- Provider API keys go to the OS credential store (`keyring`: Windows Credential
  Manager, macOS Keychain, Linux kernel keyring) under service `miyora-studio`,
  account `provider:<id>`.
- SQLite `provider_accounts` stores only that reference. A test asserts the key
  bytes are absent from `studio.sqlite` and its WAL.
- The key is never returned to the UI, never logged, and sent only as the
  `Authorization` header to the provider API host. Output downloads carry no
  credentials.
- The settings dialog uses a password field and clears it after saving.

## Untrusted input

| Input | Controls |
|---|---|
| User images | regular file only; size checked before read (20 MB); format by content sniffing (PNG/JPEG/WebP); full decode with 8192 px and 512 MB allocation limits; original copied, never modified |
| Provider JSON | 2 MB response cap; strict envelope (`HTTP 200 && code 0 && data`); task ID echo must match; task/file IDs restricted to `[A-Za-z0-9_-]` before use in URL paths |
| Provider output URLs | HTTPS only (transport is `https_only`); 512 MB model / 20 MB preview caps; URLs never logged |
| Downloaded models | must be a structurally valid GLB 2.0 (magic, version, declared length, JSON chunk ≤ 64 MB) or FBX; otherwise the task fails with `conversion` and nothing is versioned |
| Stored paths | relative paths from the DB must contain only normal components before joining the studio root |
| Names, prompts | control characters stripped; length-bounded; emoji removed from prompts |

Provider content is data only: nothing is executed, and model files are parsed
by three.js loaders in the webview.

## Tauri surface

- No new Tauri capability or plugin permission was added. File selection uses
  the existing `dialog:allow-open`; drag-and-drop uses the webview's built-in
  drop event.
- 19 narrow `studio_*` commands; no generic file, shell or HTTP command.
  Binary outputs are served by output ID, never by path.

## Export writes

- The destination comes only from the native Save dialog opened in Rust; the
  webview cannot pass a path to write. OBJ sidecars (`.mtl`, texture) are
  written next to that file using names derived from it.
- Writes go through a `.part` file and rename; outputs are re-read and verified
  before being recorded. GLB parsing is bounded (vertex/index/node limits, no
  external buffers).

## Local engine process

- Executable and runner come only from `engine.json`, and both must resolve
  inside the engine root (checked with `canonicalize`). No shell, fixed
  argument list, `python -I` (ignores `PYTHON*` env and user site-packages),
  stdin closed, output to the job's `engine.log`, 30-minute kill limit.
- The runner forces `HF_HUB_OFFLINE=1`: generation never touches the network.
- Only `model.glb` and `processed_input.png` of a job can be read back.

## Hugging Face Space

- Requests go only to `https://trellis-community-trellis.hf.space`; result
  URLs must be file URLs on that host. Server paths from uploads are checked
  (absolute, no `..`, ≤ 1024 chars). The optional token is stored in the OS
  keyring and sent only to the Space API, never with file downloads.
- Images sent to the Space leave the computer; the engine picker labels this
  provider "online".

## Logging

Structured JSON lines with operation, task ID, provider, status, error kind and
duration only. No prompts, keys, URLs, file contents or provider bodies.

## Development harness

`apps/desktop/studio3d-preview.html` mocks read-only IPC for UI work. It is
labelled `[SIMULATED - DEV ONLY]`, is not a production build entry, and cannot
contact a provider or write data.
