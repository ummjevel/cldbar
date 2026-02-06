import { useState } from "react";
import { ArrowLeft, Check, Loader2, AlertCircle, FolderOpen, Info } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { providerColors } from "../../lib/colors";
import { setDialogOpen, startManualDrag } from "../../lib/windowState";
import { apiSupportedProviders, accountSupportedProviders, type ProviderType, type SourceType } from "../../lib/types";

interface Props {
  onBack: () => void;
  onAdded: () => void;
}

const providers: { type: ProviderType; label: string }[] = [
  { type: "claude", label: "Claude" },
  { type: "gemini", label: "Gemini" },
  // { type: "zai", label: "z.ai" },  // TODO: re-enable when z.ai API is stable
];

export function AddProfileForm({ onBack, onAdded }: Props) {
  const [providerType, setProviderType] = useState<ProviderType>("claude");
  const [sourceType, setSourceType] = useState<SourceType>("account");
  const [name, setName] = useState("");
  const [configDir, setConfigDir] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [validating, setValidating] = useState(false);
  const [validated, setValidated] = useState<boolean | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const supportsApi = apiSupportedProviders.includes(providerType);
  const supportsAccount = accountSupportedProviders.includes(providerType);
  const isApi = sourceType === "api";

  // Auto-select appropriate source type when switching providers
  const handleProviderChange = (type: ProviderType) => {
    setProviderType(type);
    const canApi = apiSupportedProviders.includes(type);
    const canAccount = accountSupportedProviders.includes(type);
    if (!canApi && sourceType === "api") {
      setSourceType("account");
    } else if (!canAccount && sourceType === "account") {
      setSourceType("api");
    }
    setValidated(null);
    setError(null);
  };

  const handleValidate = async () => {
    if (!apiKey.trim()) return;
    setValidating(true);
    setValidated(null);
    setError(null);
    try {
      const valid = await invoke<boolean>("validate_api_key", { apiKey: apiKey.trim(), providerType });
      setValidated(valid);
      if (!valid) setError("Invalid API key or insufficient permissions");
    } catch (e) {
      setValidated(false);
      setError(String(e));
    } finally {
      setValidating(false);
    }
  };

  const handleSubmit = async () => {
    setError(null);

    const profileName = name.trim() || `${providers.find(p => p.type === providerType)?.label}${isApi ? " (API)" : ""}`;
    const id = `${providerType}-${isApi ? "api" : "account"}-${Date.now()}`;

    if (isApi && !apiKey.trim()) {
      setError("API key is required");
      return;
    }

    if (!isApi && !configDir.trim()) {
      setError("Config directory is required");
      return;
    }

    setSubmitting(true);
    try {
      await invoke("add_profile", {
        profile: {
          id,
          name: profileName,
          providerType: providerType,
          configDir: isApi ? "" : configDir.trim(),
          enabled: true,
          sourceType: sourceType,
          apiKey: isApi ? apiKey.trim() : null,
        },
      });
      onAdded();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const canSubmit = isApi ? (apiKey.trim().length > 0) : (configDir.trim().length > 0);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div
        className="flex items-center gap-2 px-4 py-2.5 border-b border-border cursor-grab active:cursor-grabbing"
        onMouseDown={startManualDrag}
      >
        <button
          onClick={onBack}
          className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
        >
          <ArrowLeft size={14} className="text-muted" />
        </button>
        <span className="text-sm font-semibold text-text">Add Profile</span>
      </div>

      {/* Form */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
        {/* Provider selection */}
        <div>
          <label className="text-[10px] font-medium text-muted uppercase tracking-wider mb-1.5 block">
            Provider
          </label>
          <div className="flex gap-1.5">
            {providers.map((p) => {
              const colors = providerColors[p.type];
              const active = providerType === p.type;
              return (
                <button
                  key={p.type}
                  onClick={() => handleProviderChange(p.type)}
                  className="flex-1 px-2 py-2 rounded-lg text-xs font-medium border transition-all"
                  style={{
                    borderColor: active ? colors.main : "var(--color-border)",
                    backgroundColor: active ? colors.bg : "var(--color-card)",
                    color: active ? colors.main : "var(--color-muted)",
                  }}
                >
                  {p.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Source type toggle - temporarily hidden, only account mode supported
        <div>
          <label className="text-[10px] font-medium text-muted uppercase tracking-wider mb-1.5 block">
            Source
          </label>
          <div className="flex gap-1.5">
            <button
              onClick={() => { if (supportsAccount) { setSourceType("account"); setError(null); } }}
              disabled={!supportsAccount}
              className="flex-1 px-2 py-2 rounded-lg text-xs font-medium border transition-all"
            >
              Account{!supportsAccount && " (N/A)"}
            </button>
            <button
              onClick={() => { if (supportsApi) { setSourceType("api"); setError(null); } }}
              disabled={!supportsApi}
              className="flex-1 px-2 py-2 rounded-lg text-xs font-medium border transition-all"
            >
              API{!supportsApi && " (N/A)"}
            </button>
          </div>
        </div>
        */}

        {/* Name (optional) */}
        <div>
          <label className="text-[10px] font-medium text-muted uppercase tracking-wider mb-1.5 block">
            Name <span className="normal-case">(optional)</span>
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={`${providers.find(p => p.type === providerType)?.label}${isApi ? " (API)" : ""}`}
            className="w-full px-3 py-2 rounded-lg border border-border text-xs text-text placeholder:text-muted/50 outline-none focus:border-border-light transition-colors"
            style={{ backgroundColor: "var(--theme-input-bg)" }}
          />
        </div>

        {/* API key input (for API source) */}
        {isApi && (
          <div>
            <label className="text-[10px] font-medium text-muted uppercase tracking-wider mb-1.5 block">
              API Key
            </label>
            <div className="flex gap-1.5">
              <input
                type="password"
                value={apiKey}
                onChange={(e) => { setApiKey(e.target.value); setValidated(null); }}
                placeholder={providerType === "zai" ? "your-api-key..." : "sk-ant-admin..."}
                className="flex-1 px-3 py-2 rounded-lg border border-border text-xs text-text placeholder:text-muted/50 outline-none focus:border-border-light transition-colors font-mono"
                style={{ backgroundColor: "var(--theme-input-bg)" }}
              />
              <button
                onClick={handleValidate}
                disabled={validating || !apiKey.trim()}
                className="px-3 py-2 rounded-lg text-xs font-medium border border-border bg-card hover:bg-card-hover transition-colors disabled:opacity-40"
                style={{
                  color: validated === true ? "#22c55e" : validated === false ? "#ef4444" : "var(--color-text-secondary)",
                }}
              >
                {validating ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : validated === true ? (
                  <Check size={12} />
                ) : validated === false ? (
                  <AlertCircle size={12} />
                ) : (
                  "Test"
                )}
              </button>
            </div>
            <p className="text-[10px] text-muted mt-1">
              {providerType === "zai"
                ? "API key from z.ai/manage-apikey"
                : "Requires an Admin API key from console.anthropic.com"}
            </p>
          </div>
        )}

        {/* Config dir input (for Account source) */}
        {!isApi && (
          <div>
            <label className="text-[10px] font-medium text-muted uppercase tracking-wider mb-1.5 block">
              Config Directory
            </label>
            <div className="flex gap-1.5">
              <input
                type="text"
                value={configDir}
                onChange={(e) => setConfigDir(e.target.value)}
                placeholder={
                  providerType === "claude" ? "C:\\Users\\...\\.claude"
                  : providerType === "gemini" ? "C:\\Users\\...\\.gemini"
                  : "%APPDATA%\\zai"
                }
                className="flex-1 px-3 py-2 rounded-lg border border-border text-xs text-text placeholder:text-muted/50 outline-none focus:border-border-light transition-colors font-mono"
                style={{ backgroundColor: "var(--theme-input-bg)" }}
              />
              <button
                onClick={async () => {
                  setDialogOpen(true);
                  try {
                    const selected = await open({ directory: true, title: "Select config directory" });
                    if (selected) setConfigDir(selected as string);
                  } finally {
                    setDialogOpen(false);
                  }
                }}
                className="px-3 py-2 rounded-lg text-xs font-medium border border-border bg-card hover:bg-card-hover transition-colors"
              >
                <FolderOpen size={12} className="text-muted" />
              </button>
            </div>
          </div>
        )}

        {/* Gemini notice */}
        {providerType === "gemini" && (
          <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-card border border-border">
            <Info size={11} className="text-muted shrink-0 mt-0.5" />
            <p className="text-[10px] text-muted leading-relaxed">
              Only tracks local Gemini CLI usage. For full usage details, visit{" "}
              <span className="text-text-secondary">aistudio.google.com</span>.
            </p>
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-danger/10 border border-danger/20">
            <AlertCircle size={12} className="text-danger shrink-0 mt-0.5" />
            <p className="text-[10px] text-danger">{error}</p>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-border">
        <button
          onClick={handleSubmit}
          disabled={!canSubmit || submitting}
          className="w-full py-2 rounded-lg text-xs font-semibold transition-all disabled:opacity-40"
          style={{
            backgroundColor: providerColors[providerType].main,
            color: "#fff",
          }}
        >
          {submitting ? "Adding..." : "Add Profile"}
        </button>
      </div>
    </div>
  );
}
