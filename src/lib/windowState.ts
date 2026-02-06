import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";

// Module-level flags to prevent window hide during native dialogs or dragging
let _dialogOpen = false;
let _dragging = false;

export function isDialogOpen() {
  return _dialogOpen;
}

export function setDialogOpen(value: boolean) {
  _dialogOpen = value;
}

export function isDragging() {
  return _dragging;
}

export function setDragging(value: boolean) {
  _dragging = value;
}

/**
 * Start manual window drag from a mousedown event.
 * Uses mouse tracking + setPosition instead of startDragging()
 * which doesn't work on Windows transparent windows.
 */
export function startManualDrag(e: React.MouseEvent) {
  if (e.buttons !== 1) return;
  if ((e.target as HTMLElement).closest("button")) return;

  e.preventDefault();
  setDragging(true);

  const startX = e.screenX;
  const startY = e.screenY;
  const win = getCurrentWindow();

  win.outerPosition().then((pos) => {
    const startWinX = pos.x;
    const startWinY = pos.y;
    const scale = window.devicePixelRatio || 1;

    const onMouseMove = (ev: MouseEvent) => {
      const dx = (ev.screenX - startX) * scale;
      const dy = (ev.screenY - startY) * scale;
      win.setPosition(new PhysicalPosition(startWinX + dx, startWinY + dy));
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      setTimeout(() => setDragging(false), 200);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  });
}
