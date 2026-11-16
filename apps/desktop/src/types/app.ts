import type { PetPresentation } from "../avatar/types";
import type { PetState } from "../components/CompanionRenderer";

export interface RuntimeSnapshot {
  petState: PetState;
  selectedPetId: string | null;
  lastEventAt: string | null;
  isSimulated?: boolean;
  presentation?: PetPresentation;
}

export interface ProgressionSnapshot {
  petId: string;
  totalXp: number;
  level: number;
  xpIntoLevel: number;
  xpToNextLevel: number;
}

export interface XpTransaction {
  transactionId: string;
  petId: string;
  source: string;
  sourceId: string;
  amount: number;
  createdAt: string;
  metadataJson: string;
}

export interface AgentSessionSummary {
  sessionId: string;
  agentId: string;
  projectPath: string | null;
  startedAt: string;
  endedAt: string | null;
  durationMs: number | null;
  status: "running" | "completed" | "failed" | "interrupted";
  exitCode: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface AchievementSummary {
  id: string;
  title: string;
  description: string;
  threshold: number;
  progress: number;
  rewardXp: number;
  unlockedAt: string | null;
}

export interface PetPackSummary {
  id: string;
  selectionId: string;
  version: string;
  name: string;
  creator: string;
  license: string;
  previewDataUrl: string | null;
}

export interface StoredAgentEvent {
  id: string;
  eventType: string;
  occurredAt: string;
  source: string;
  provider: string;
  sessionId: string | null;
  projectId: string | null;
  payloadJson: string | null;
  latencyMs: number | null;
}

export interface LedgerVerificationResult {
  isValid: boolean;
  totalEntries: number;
  headHash: string;
  brokenAtIndex: number | null;
  errorMessage: string | null;
}

export type AppTab = "den" | "sessions" | "companions" | "progress" | "settings";

export const APP_TABS: readonly { id: AppTab; label: string; icon: string }[] = [
  { id: "den", label: "Den", icon: "🏛️" },
  { id: "sessions", label: "Sessions", icon: "📊" },
  { id: "companions", label: "Pet Library", icon: "✨" },
  { id: "progress", label: "Progress", icon: "🔒" },
  { id: "settings", label: "Settings", icon: "⚙️" },
];
