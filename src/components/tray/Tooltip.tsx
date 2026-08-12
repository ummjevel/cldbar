import { motion } from "framer-motion";

interface Props {
  text: string;
  /**
   * Which side of the anchor to sit on. The content area scrolls, so an
   * element at the very top has no room above it and must point down.
   */
  placement?: "top" | "bottom";
}

/** Small hover explainer. The anchor must be `relative`. */
export function Tooltip({ text, placement = "top" }: Props) {
  const isTop = placement === "top";

  return (
    <motion.div
      initial={{ opacity: 0, y: isTop ? 4 : -4 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: isTop ? 4 : -4 }}
      transition={{ duration: 0.15 }}
      className={`absolute left-1/2 -translate-x-1/2 px-2 py-1 rounded-md text-[9px] text-text whitespace-nowrap z-10 border border-border-light pointer-events-none ${
        isTop ? "bottom-full mb-1.5" : "top-full mt-1.5"
      }`}
      style={{ backgroundColor: "var(--theme-bg)", backdropFilter: "blur(12px)" }}
    >
      {text}
    </motion.div>
  );
}
