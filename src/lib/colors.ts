import { ProviderType } from "./types";

export const providerColors: Record<ProviderType, { main: string; light: string; bg: string }> = {
  claude: { main: "#d97706", light: "#f59e0b", bg: "rgba(217, 119, 6, 0.1)" },
  gemini: { main: "#4285f4", light: "#60a5fa", bg: "rgba(66, 133, 244, 0.1)" },
  zai: { main: "#10b981", light: "#34d399", bg: "rgba(16, 185, 129, 0.1)" },
};

export const providerLabels: Record<ProviderType, string> = {
  claude: "Claude",
  gemini: "Gemini",
  zai: "z.ai",
};
