# Provider API

## `GenerationProvider` (crates/studio/src/provider.rs)

```rust
pub trait GenerationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> &'static [Capability];   // only what is implemented
    fn models(&self) -> Vec<ModelInfo>;
    fn default_model(&self) -> &'static str;
    fn limits(&self) -> ProviderLimits;                 // prompt/negative/image/quad limits
    fn upload_image(&self, bytes, file_name, mime) -> Result<String /*file token*/, ProviderError>;
    fn text_to_3d(&self, &TextTo3dRequest) -> Result<String /*task id*/, ProviderError>;
    fn image_to_3d(&self, &ImageTo3dRequest) -> Result<String, ProviderError>;
    fn task_status(&self, task_id) -> Result<ProviderTask, ProviderError>;
    fn account_balance(&self) -> Result<AccountBalance, ProviderError>;
    fn download(&self, url, max_bytes) -> Result<Vec<u8>, ProviderError>;
}
```

`Capability` also names `MultiviewTo3d`, `TextToImage`, `ImageToImage`,
`Texture`, `Segmentation`, `MeshCompletion`, `Retopology`, `RigCheck`, `Rig`,
`Retarget`, `Convert`, `TaskList` for future providers; Tripo advertises only
`TextTo3d`, `ImageTo3d`, `TaskStatus`, `Upload`, `AccountBalance`.

Each provider also declares `label`, `requires_api_key`, `accepts_api_key`
(optional keys), `availability()` (e.g. local engine not installed) and
`supported_settings()`. The service rejects any set `GenerationSettings` field a
provider does not support, and the UI only shows supported controls.

Adding a provider: implement the trait, add its id to `PROVIDER_IDS` and a
branch in `provider_factory` (`service.rs`).

Providers in this build: `local-triposr` (default when installed), `hf-trellis`,
`tripo`. The UI reads capabilities, models
and limits from `studio_overview`; it never calls a provider.

## Tripo v3 mapping (`crates/studio/src/tripo.rs`)

Base URL `https://openapi.tripo3d.ai/v3`, header `Authorization: Bearer <key>`.

| Operation | Endpoint | Body |
|---|---|---|
| Upload | `POST /files` (multipart `file`) | → `data.file_token` |
| Text → 3D | `POST /generation/text-to-model` | `prompt`, `negative_prompt`, `model`, seeds, `face_limit`, `texture`, `pbr`, `texture_quality`, `geometry_quality`, `auto_size`, `quad`, `smart_low_poly`, `export_uv`, `compress:"geometry"`, `export_orientation` |
| Image → 3D | `POST /generation/image-to-model` | `input: <file_token>` + the settings above |
| Task | `GET /tasks/{task_id}` | → `status`, `progress`, `output.*_url`, `credits_consumed` |
| Balance | `GET /account/balance` | → `balance`, `frozen` |

Unset settings are omitted so the provider default applies. Seeds left blank in
the UI are generated locally and sent, so every generation is reproducible.

### Success vs. failure

Only `HTTP 200 && code == 0 && data != null` is success. A `task_id` means the
request was accepted, not that a model exists.

### Error mapping

| Provider code / HTTP | `ErrorKind` |
|---|---|
| 1000, 1001, HTTP 401/403 | `authentication` |
| 1007, 2000, HTTP 429 (Retry-After honoured) | `rate_limit` |
| 2010 | `insufficient_credits` |
| 1004, 2002, 2003, 2004, 2008, 2015, 2018, HTTP 400/404/422 | `validation` |
| HTTP 5xx, connect/timeout | `provider_unavailable` |
| upload rejected | `file_upload` |
| download/validation of outputs failed | `conversion` |
| unreadable / unexpected response | `unknown_provider` |

`task_failed` and `task_timeout` are produced by the task system. Provider
messages are truncated to 300 characters; request IDs are kept.

### Request validation before any network call

Prompt 1–1024 chars, negative ≤ 255, model id `[A-Za-z0-9._-]{1,64}`, face limit
≤ model max (v3.1: 1.5 M standard / 2 M detailed; v3.0: 1 M / 2 M; v2.5: 500 k),
quad ≤ 150 000, smart low-poly 500–20 000, v3-only options rejected on v2.5,
task IDs `[A-Za-z0-9_-]{1,128}`.

## Tests

`crates/studio/src/tripo.rs` tests replay the response examples in
`crates/studio/tests/fixtures/tripo_v3/` (copied from the public docs) through a
test-only `ScriptedTransport`. Production code has no fake transport.

## Local TripoSR (`crates/studio/src/local.rs`)

- Install once: `python engines/triposr/setup_engine.py` (uv + git required;
  ~4.5 GB). Creates `%LOCALAPPDATA%/Miyora/engines/triposr` with TripoSR at a
  pinned commit, a Python 3.11 venv (CUDA PyTorch), cached weights and
  `engine.json` (paths relative to the engine root). `MIYORA_ENGINE_ROOT`
  overrides the location.
- `upload_image` stages the image in `<studio>/local-jobs/<job>/input.png`;
  `image_to_3d` queues the job. One worker thread runs one job at a time
  (single GPU) as `python -I run_triposr.py --job-dir <job> ...` with
  `HF_HUB_OFFLINE=1`, no shell, no console window, 30-minute kill limit.
- The runner writes `status.json` (`queued/running/succeeded/failed`, stage,
  device, error). `task_status` reports only that; progress is never invented.
  Jobs still `running` after an app restart are reported as interrupted.
- `download` accepts only `local://<job>/model.glb` and
  `local://<job>/processed_input.png`.
- Settings: `texture: true` bakes a UV texture (xatlas + moderngl); otherwise
  vertex colours. Chunk size 2048 and marching-cubes resolution 256 (fits 4 GB).
- `torchmcubes` (needs a C++/CUDA build) is replaced by a PyMCubes shim
  (`engines/triposr/shim/torchmcubes.py`); the runner flips faces if the mesh
  volume is negative and rotates z-up to glTF y-up.

## TRELLIS on Hugging Face (`crates/studio/src/hf.rs`)

Space `trellis-community/TRELLIS` (Gradio 5.34, `sse_v3`). Per job, one
`session_hash`:

1. `GET /config` → resolve `fn_index` for `start_session`, `preprocess_image`,
   `generate_and_extract_glb` by `api_name`; refuse if `generate` no longer has
   11 inputs.
2. `POST /gradio_api/upload` (done in `upload_image`).
3. `POST /gradio_api/queue/join` for each step, then read
   `GET /gradio_api/queue/data?session_hash=…` until `process_completed`.
4. Generate inputs: preprocessed image, `[]`, `null` (multi-image state), seed,
   7.5, 12, 3.0, 12, `"stochastic"`, 0.95, 1024 (the Space's defaults).
5. The GLB `url` must start with `https://trellis-community-trellis.hf.space/gradio_api/file=`.

Errors: HTTP 401/403 → token rejected; 429 → rate limit; 5xx → Space sleeping
or busy; `process_completed` with `success:false` → Space error (usually the free
GPU quota). An optional HF token is sent as `Authorization: Bearer` to the Space
only, never to file downloads. The `/gradio_api/call` route was not used because
it does not keep the session that TRELLIS needs (observed: `404: Session not found`).
