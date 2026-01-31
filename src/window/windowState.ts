import {
  getCurrentWindow,
  currentMonitor,
  primaryMonitor,
  PhysicalSize,
  PhysicalPosition,
} from "@tauri-apps/api/window";

type StoredWindowState = {
  version: 1;
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
};

const STORAGE_KEY = "endcat.windowState.v1";

function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);
}

function safeParse(raw: string | null): StoredWindowState | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<StoredWindowState>;
    if (parsed.version !== 1) return null;
    if (
      typeof parsed.x !== "number" ||
      typeof parsed.y !== "number" ||
      typeof parsed.width !== "number" ||
      typeof parsed.height !== "number" ||
      typeof parsed.maximized !== "boolean"
    ) {
      return null;
    }
    return parsed as StoredWindowState;
  } catch {
    return null;
  }
}

function clamp(n: number, min: number, max: number) {
  return Math.max(min, Math.min(max, n));
}

export async function initWindowState() {
  if (!isTauri()) return;

  const win = getCurrentWindow();

  const monitor = (await currentMonitor()) ?? (await primaryMonitor());

  async function centerWindow() {
    if (!monitor) return;
    const workW = monitor.workArea.size.width;
    const workH = monitor.workArea.size.height;
    const width = 1175;
    const height = 875;
    const x = monitor.workArea.position.x + Math.floor((workW - width) / 2);
    const y = monitor.workArea.position.y + Math.floor((workH - height) / 2);
    try {
      await win.setSize(new PhysicalSize(width, height));
      await win.setPosition(new PhysicalPosition(x, y));
    } catch {
      // 居中失败直接忽略
    }
  }

  const storedRaw = localStorage.getItem(STORAGE_KEY);
  let stored = safeParse(storedRaw);
  if (stored && monitor) {
    const area = monitor.workArea;
    const within =
      stored.x >= area.position.x - 20 &&
      stored.y >= area.position.y - 20 &&
      stored.x + stored.width <= area.position.x + area.size.width + 20 &&
      stored.y + stored.height <= area.position.y + area.size.height + 20;
    if (!within) {
      stored = null;
    }
  }

  if (stored) {
    try {
      await win.setSize(new PhysicalSize(stored.width, stored.height));
      await win.setPosition(new PhysicalPosition(stored.x, stored.y));
      if (stored.maximized) await win.maximize();
    } catch {
      // 恢复失败则清空记录并居中
      localStorage.removeItem(STORAGE_KEY);
      await centerWindow();
    }
  } else {
    await centerWindow();
  }

  let last: StoredWindowState | null = null;
  let saveTimer: number | null = null;
  let maximizeTimer: number | null = null;
  let maximizeInFlight = false;

  async function initSnapshot() {
    const [pos, size, maximized] = await Promise.all([win.outerPosition(), win.innerSize(), win.isMaximized()]);
    last = {
      version: 1,
      x: pos.x,
      y: pos.y,
      width: size.width,
      height: size.height,
      maximized,
    };
  }

  function scheduleSave() {
    if (saveTimer != null) window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
      saveTimer = null;
      if (!last) return;
      if (last.width <= 0 || last.height <= 0) return;
      const bounded = {
        ...last,
        width: clamp(last.width, 675, 99999),
        height: clamp(last.height, 475, 99999),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(bounded));
    }, 300);
  }

  function scheduleMaximizedRefresh() {
    if (maximizeTimer != null) window.clearTimeout(maximizeTimer);
    maximizeTimer = window.setTimeout(async () => {
      maximizeTimer = null;
      if (!last) return;
      if (maximizeInFlight) return;
      maximizeInFlight = true;
      try {
        last.maximized = await win.isMaximized();
      } catch {
        // 忽略：避免任何平台因为 maximize 查询导致事件阻塞
      } finally {
        maximizeInFlight = false;
      }
    }, 500);
  }

  try {
    await initSnapshot();
  } catch {
    // 初始化采集失败时，不要影响窗口交互
    last = null;
  }

  const unlistenResized = await win.onResized(({ payload: size }) => {
    if (!last) {
      last = { version: 1, x: 0, y: 0, width: size.width, height: size.height, maximized: false };
    } else {
      last.width = size.width;
      last.height = size.height;
    }
    scheduleSave();
    scheduleMaximizedRefresh();
  });

  const unlistenMoved = await win.onMoved(({ payload: pos }) => {
    if (!last) {
      last = { version: 1, x: pos.x, y: pos.y, width: 0, height: 0, maximized: false };
    } else {
      last.x = pos.x;
      last.y = pos.y;
    }
    scheduleSave();
  });

  const unlistenClose = await win.onCloseRequested(() => {
    scheduleMaximizedRefresh();
    scheduleSave();
  });

  window.addEventListener("beforeunload", () => {
    if (!last) return;
    if (last.width <= 0 || last.height <= 0) return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(last));
  });

  return () => {
    unlistenResized();
    unlistenMoved();
    unlistenClose();
  };
}
