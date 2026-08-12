import { ProviderType } from "./types";

export const providerColors: Record<ProviderType, { main: string; light: string; bg: string }> = {
  claude: { main: "var(--provider-claude)", light: "var(--provider-claude-light)", bg: "var(--provider-claude-bg)" },
  codex: { main: "var(--provider-codex)", light: "var(--provider-codex-light)", bg: "var(--provider-codex-bg)" },
  gemini: { main: "var(--provider-gemini)", light: "var(--provider-gemini-light)", bg: "var(--provider-gemini-bg)" },
  zai: { main: "var(--provider-zai)", light: "var(--provider-zai-light)", bg: "var(--provider-zai-bg)" },
};

export const providerLabels: Record<ProviderType, string> = {
  claude: "Claude",
  codex: "Codex",
  gemini: "Gemini",
  zai: "z.ai",
};
