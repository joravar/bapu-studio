// Wraps localStorage for values that hold secrets (DB passwords, API keys, auth tokens).
// In Electron, the stored value is encrypted at rest via the main process's OS-keychain-backed
// safeStorage before it ever touches localStorage. In a plain browser (dev server, no bridge),
// there is no OS keychain to use, so it falls back to storing the value as-is.
function getBridge(): any {
  if (typeof window !== 'undefined' && (window as any).bapuBridge) {
    return (window as any).bapuBridge;
  }
  return null;
}

export function secureGetItem(key: string): string | null {
  const raw = localStorage.getItem(key);
  if (raw === null) return null;
  const bridge = getBridge();
  if (bridge?.secureDecrypt) {
    try {
      return bridge.secureDecrypt(raw);
    } catch {
      return raw;
    }
  }
  return raw;
}

export function secureSetItem(key: string, value: string): void {
  const bridge = getBridge();
  if (bridge?.secureEncrypt) {
    try {
      localStorage.setItem(key, bridge.secureEncrypt(value));
      return;
    } catch {}
  }
  localStorage.setItem(key, value);
}
