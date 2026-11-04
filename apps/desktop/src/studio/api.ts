import { invoke } from "@tauri-apps/api/core";
import { invokeSafe } from "../utils/ipc";
import { modelResponseBytes } from "../avatar/modelTransport";
import { parseStudioError } from "./format";
import type {
  AccountBalance,
  AssetDetail,
  AssetSummary,
  BuiltPrompt,
  ErrorReport,
  ExportOptions,
  ExportView,
  ImageInspection,
  ImageSubmission,
  LocalExportFormat,
  PollPolicy,
  ProjectView,
  PromptIntent,
  StudioOverview,
  TaskView,
  TextSubmission,
} from "./types";

export type StudioResult<T> = { ok: true; data: T } | { ok: false; error: ErrorReport };

/** Matches MAX_MODEL_DOWNLOAD_BYTES in crates/studio/src/service.rs. */
const MAX_OUTPUT_BYTES = 512 * 1024 * 1024;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<StudioResult<T>> {
  const result = await invokeSafe<T>(command, args);
  return result.ok ? { ok: true, data: result.data } : { ok: false, error: parseStudioError(result.error) };
}

export const studioApi = {
  overview: () => call<StudioOverview>("studio_overview"),
  saveApiKey: (provider: string, apiKey: string) => call<null>("studio_save_api_key", { provider, apiKey }),
  deleteApiKey: (provider: string) => call<null>("studio_delete_api_key", { provider }),
  accountBalance: (provider: string) => call<AccountBalance>("studio_account_balance", { provider }),
  setDefaultModel: (provider: string, model: string) => call<null>("studio_set_default_model", { provider, model }),
  setPollPolicy: (policy: PollPolicy) => call<PollPolicy>("studio_set_poll_policy", { policy }),
  buildPrompt: (provider: string, intent: PromptIntent, presetId: string | null) =>
    call<BuiltPrompt>("studio_build_prompt", { provider, intent, presetId }),
  listProjects: () => call<ProjectView[]>("studio_list_projects"),
  createProject: (name: string) => call<ProjectView>("studio_create_project", { name }),
  listAssets: (projectId: string) => call<AssetSummary[]>("studio_list_assets", { projectId }),
  assetDetail: (assetId: string) => call<AssetDetail>("studio_asset_detail", { assetId }),
  listTasks: () => call<TaskView[]>("studio_list_tasks"),
  inspectImage: (provider: string, path: string) => call<ImageInspection>("studio_inspect_image", { provider, path }),
  submitText: (submission: TextSubmission) => call<TaskView>("studio_submit_text", { submission }),
  submitImage: (submission: ImageSubmission) => call<TaskView>("studio_submit_image", { submission }),
  regenerate: (generationId: string) => call<TaskView>("studio_regenerate", { generationId }),
  resumeTask: (taskId: string) => call<TaskView>("studio_resume_task", { taskId }),
  /** Opens the native Save dialog; resolves to null when the user cancels. */
  exportOutput: (outputId: number, format: LocalExportFormat, options: ExportOptions) =>
    call<ExportView | null>("studio_export_output", { outputId, format, options }),
  retopologize: (generationId: string, targetTriangles: number, preserveBorders: boolean) =>
    call<TaskView>("studio_retopologize", { request: { generationId, targetTriangles, preserveBorders } }),
};

/** Raw output bytes; uses invoke directly because the payload is binary, not JSON. */
export async function readOutput(outputId: number): Promise<ArrayBuffer> {
  try {
    return modelResponseBytes(await invoke<unknown>("studio_read_output", { outputId }), MAX_OUTPUT_BYTES);
  } catch (error: unknown) {
    const message = typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
    throw new Error(parseStudioError(message).message);
  }
}
