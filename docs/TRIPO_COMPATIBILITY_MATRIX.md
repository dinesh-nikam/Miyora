# Tripo Compatibility Matrix — Miyora 3D Studio

Miyora 3D Studio is an independent application. Tripo is used only as a
provider through its **publicly documented API v3**
(`https://openapi.tripo3d.ai/v3`, docs at https://developers.tripo3d.ai/en/docs,
retrieved 2026-10-29). No Tripo source, private endpoint, binary, or credential
was inspected or used. Tripo API v2 is retired on 2026-11-01 and is not used.

## Engines (providers)

| Provider id | Engine | Cost / access | Verified for real |
|---|---|---|---|
| `local-triposr` | TripoSR (MIT, Tripo AI & Stability AI) on this computer | Free, offline after one-time setup (`engines/triposr/setup_engine.py`) | Yes — image→3D on an NVIDIA T500 (4 GB): 442 s, 44,312 triangles, watertight, y-up, vertex colours |
| `hf-trellis` | TRELLIS (MIT, Microsoft) on the public Hugging Face Space `trellis-community/TRELLIS` | Free ZeroGPU quota (anonymous or with a free HF token); needs internet | Yes — image→3D: 21 s, 7,569 triangles, UV + base-colour texture |
| `tripo` | Tripo API v3 (optional) | Paid credits | No — fixture-tested only (0 credits) |

Generation quality of the free engines is below Tripo's commercial models.

## Status legend

- **IMPLEMENTED** — built and verified by automated tests and/or the UI harness.
- **PARTIAL** — built, but part of the feature or its live verification is missing (see Limitations).
- **UNAVAILABLE** — not built in this version (the provider may document it).
- **NOT APPLICABLE** — the provider has no documented contract for it.

> **Live-provider caveat (Tripo only).** No Tripo call has been made against the
> live API: the available account had a 0 credit balance. Tripo-only rows are
> verified against response fixtures copied from the public docs
> (`crates/studio/tests/fixtures/tripo_v3`). The local and Hugging Face engines
> were exercised for real.

## Matrix

| Feature | Observed behaviour (public docs) | Public API support | Status | Provider | Local implementation | Limitations | Tests |
|---|---|---|---|---|---|---|---|
| Text → 3D | Async task: POST returns `task_id`, poll until `success` | `POST /generation/text-to-model` | PARTIAL | tripo | `tripo.rs` `text_to_3d`, `service.rs` `submit_text_to_3d`, `CreatePanel.tsx` | Live call unverified (no credits). P-series model (`P1-20260311`) not modelled | `tripo::tests::text_to_3d_posts_documented_fields_*`, `service::tests::text_to_3d_end_to_end_*` |
| Image → 3D | Upload image → task → model | `POST /files`, `POST /generation/image-to-model` | IMPLEMENTED (local, HF) / PARTIAL (tripo) | local-triposr, hf-trellis, tripo | `local.rs`, `hf.rs`, `tripo.rs`; `service.rs` `submit_image_to_3d`; file picker + drag-and-drop | Clipboard paste not supported. Crop not implemented. Background removal only inside the engines (rembg / TRELLIS preprocessing). Tripo path not exercised live | `local::tests::*`, `hf::tests::*`, `service::tests::image_to_3d_*`; real runs via `cargo run -p agentpet-studio --example generate` |
| Image validation | PNG/JPEG/WebP ≤ 20 MB, ≥ 256 px recommended | — | IMPLEMENTED | local | `media.rs` `validate_image` (content sniffing, full decode, size, dimension limits) | `/files` docs list JPEG/PNG only, so WebP is converted to a PNG working copy before upload | `media::tests::*image*` |
| Multiview → 3D | Views keyed front/left/back/right, front required, ≥ 2 views | `POST /generation/multiview-to-model` | PARTIAL | tripo | `tripo.rs` `multiview_to_3d` request mapping only | No service method or UI yet; local/HF engines do not offer it | `tripo::tests::multiview_uses_view_key_inputs_*` |
| Text → Image / Image → Image | Concept image tasks | `POST /generation/text-to-image`, `/image-to-image` | UNAVAILABLE | — | — | Not built | — |
| Prompt builder | Prompt ≤ 1024 chars, negative ≤ 255, no emoji | — | IMPLEMENTED | local | `prompt.rs` structured intent → bounded, sanitized prompt; final prompt editable in UI | — | `prompt::tests::*` |
| Presets (12) | — | — | IMPLEMENTED | local | `prompt.rs` `PRESETS` (provider-neutral defaults) | — | `prompt::tests::twelve_unique_presets` |
| Model configuration | Models listed in docs; no list endpoint | none (docs only) | IMPLEMENTED | tripo | `documented_models()`, default model setting | List is static from docs (`source: "documentation"`) | `service::tests::managed_paths_cannot_escape_root_and_settings_validate` |
| 3D viewer | — | — | PARTIAL | local | `ModelViewport.tsx`: GLB (+meshopt), FBX; orbit/pan/zoom, frame, reset, grid, env lighting, material/solid/wireframe, bounds, stats | No separate .gltf/OBJ/USDZ loading, no Draco decoder, no transform gizmo, no part selection | `studio.test.ts` (`computeSceneStats`); manual: dev harness |
| Model inspector | — | — | IMPLEMENTED | local | `Inspector.tsx` from stored generation record + GLB JSON report + loaded scene | Values the file does not contain show "Unknown" / "—" | `media::tests::inspects_glb_*` |
| Task system | States queued/running/success/failed/banned/expired/cancelled | `GET /tasks/{task_id}` | IMPLEMENTED | tripo | `task.rs`, `store.rs`, `service.rs` `run_poller` | — | `task::tests::*`, `store::tests::*`, `service::tests::offline_*`, `timeout_*` |
| Task cancellation | — | No cancel endpoint documented | NOT APPLICABLE | — | — | UI offers no cancel | — |
| Webhooks | Supported by provider | Webhook docs exist | UNAVAILABLE | — | — | Needs a reachable server; desktop app polls instead | — |
| Local library (projects, assets, versions, history) | — | — | IMPLEMENTED | local | `studio.sqlite` + managed files; `LibraryPanel.tsx`, `Inspector.tsx` | Tags stored but not editable; no folders | `store::tests::*` |
| Versioning & lineage | — | — | IMPLEMENTED | local | `asset_versions` with `parent_version_id`; "Generate new version" | No side-by-side comparison | `store::tests::versions_number_*`, `service::tests::text_to_3d_end_to_end_*` |
| Credits / balance | Freeze-then-deduct; `credits_consumed` on task | `GET /account/balance` | PARTIAL | tripo | Balance in settings; provider-reported credits stored per task (`source='provider'`) | No local estimates; `/account/usage` not used | `tripo::tests::upload_and_balance_*` |
| Error mapping | Unified `{code,message,suggestion,request_id}` | Error-code table | IMPLEMENTED | tripo | `tripo.rs` `error_kind` → 10 stable kinds | — | `tripo::tests::maps_documented_errors_*` |
| Credentials | Bearer API key | `Authorization: Bearer` | IMPLEMENTED | local | OS credential store via `keyring`; SQLite stores `provider:tripo` reference only | Real keyring backend not exercised in automated tests (in-memory test double) | `service::tests::api_key_lives_only_in_credential_store` |
| Texture / re-texture | — | `POST /models/texture` | UNAVAILABLE | — | — | Not built | — |
| Retopology | — | `POST /mesh/decimate` | UNAVAILABLE | — | — | Not built | — |
| Segmentation | — | `POST /mesh/segment` | UNAVAILABLE | — | — | Not built | — |
| Mesh completion | — | `POST /mesh/complete` | UNAVAILABLE | — | — | Not built | — |
| Rig check / rig / retarget | — | `POST /animations/rig-check`, `/rig`, `/retarget` | UNAVAILABLE | — | — | Not built; inspector only reports skins/animations present in the file | — |
| Animation playback | — | — | UNAVAILABLE | — | — | Not built | — |
| Conversion / export | Async convert task to GLTF/FBX/USDZ/OBJ/STL/3MF | `POST /models/convert` | IMPLEMENTED (local OBJ/STL/PLY/original) / PARTIAL (cloud) | local, tripo | `export.rs` + native Save dialog; verified re-read; `exports` table (schema v2). Tripo `convert` request mapping + `media::inspect_export` | No FBX/USDZ/3MF output locally; cloud conversion has no UI yet | `export::tests::*`, `service::tests::export_writes_verifies_and_records_every_format`, `store::tests::version_one_database_upgrades_to_two`; real files checked with trimesh |
| Batch / comparison | — | — | UNAVAILABLE | — | — | Not built | — |
| Offline use | — | — | IMPLEMENTED | local | Startup makes no network call; library and viewer read local files | Pending tasks stop after the network-error budget and can be resumed | `service::tests::offline_polling_*` |

## Behaviour contracts (implemented flows)

**Text → 3D**
INPUT prompt (+ optional structured intent, negative prompt, settings) →
VALIDATION non-empty, ≤ 1024 / ≤ 255 chars, model id charset, face limits per
model/quality, quad ≤ 150 000, smart low-poly 500–20 000, v3-only options
rejected for v2.5 → USER ACTION Generate (Ctrl+Enter) → API
`POST /v3/generation/text-to-model` → TASK STATE `QUEUED` → `RUNNING`
(`generating`) → `RUNNING` (`downloading`) → `SUCCEEDED` only after the model
file is downloaded and validated → OUTPUT GLB/FBX + preview PNG in managed
storage, version N → ERROR STATES `FAILED` with stable kind, request ID,
suggestion; `EXPIRED`; resumable when transient → UX task row, inspector,
viewport.

**Image → 3D**
Same as above, with: INPUT PNG/JPEG/WebP file → VALIDATION content sniff,
full decode, ≤ 20 MB, ≤ 8192 px, warning < 256 px → original copied untouched to
`studio/inputs/<generation>/source.<ext>` → `POST /v3/files` (WebP sent as PNG
working copy) → `POST /v3/generation/image-to-model` with `input = file_token`.

## Open uncertainties (recorded, not guessed)

- `/v3/files` lists JPEG/PNG, image-to-model lists PNG/JPEG/WebP. Resolved
  locally by uploading WebP as PNG.
- Output keys other than `model_url` and `rendered_image_url` are not
  documented. Any `*_url` HTTPS key is accepted; keys containing `image` are
  treated as previews, all others as models that must pass validation.
- Failed-task error text field is not documented; `error` is read when present,
  otherwise a generic message is shown.
