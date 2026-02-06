import { useEffect } from "react";
import { TrayPopup } from "./components/tray/TrayPopup";
import { useSettings } from "./hooks/useProviderData";
import { applyTheme } from "./lib/theme";

export default function App() {
  const { settings } = useSettings();

  useEffect(() => {
    const theme = settings?.theme || "system";
    applyTheme(theme);

    // Listen for OS theme changes when in system mode
    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const onChange = () => applyTheme("system");
      mq.addEventListener("change", onChange);
      return () => mq.removeEventListener("change", onChange);
    }
  }, [settings?.theme]);

  return (
    <div className="h-screen bg-transparent">
      <TrayPopup />
    </div>
  );
}
