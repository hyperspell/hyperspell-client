// Typed wrappers over the Rust IPC commands (see src-tauri/src/ipc.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

// Open the device-login approval page in the system browser. `login --json` is
// auth-only and deliberately does NOT open a browser — the app does.
export const openLoginUrl = (url: string) => openUrl(url);

export interface Permissions {
  claude_code: boolean;
  codex: boolean;
  cursor: boolean;
  claude_desktop: boolean;
  visible_mirror: boolean;
}

export interface Status {
  logged_in: boolean;
  identity: string | null;
  daemon_installed: boolean;
  daemon_running: boolean;
  permissions: Permissions;
}

// The integrations the user consents to, in the order the consent screen shows
// them. `key` is the daemon's flat config key; `permKey` is the Permissions field.
export const INTEGRATIONS: {
  key: string;
  permKey: keyof Permissions;
  label: string;
  detail: string;
}[] = [
  {
    key: "perm_claude_code",
    permKey: "claude_code",
    label: "Claude Code",
    detail: "Adds a Hyperspell section to ~/.claude/CLAUDE.md",
  },
  {
    key: "perm_codex",
    permKey: "codex",
    label: "Codex",
    detail: "Adds a Hyperspell section to ~/.codex/AGENTS.md",
  },
  {
    key: "perm_cursor",
    permKey: "cursor",
    label: "Cursor",
    detail: "Adds a Hyperspell section to ~/.cursorrules",
  },
  {
    key: "perm_claude_desktop",
    permKey: "claude_desktop",
    label: "Claude Desktop",
    detail: "Registers the hyperbrain MCP server in Claude Desktop",
  },
  {
    key: "perm_visible_mirror",
    permKey: "visible_mirror",
    label: "World-readable mirror",
    detail: "Mirrors synced docs to ~/Hyperspell (readable by other tools)",
  },
];

export const getStatus = () => invoke<Status>("get_status");
export const getPermissions = () => invoke<Permissions>("get_permissions");
export const setPermission = (key: string, value: boolean) =>
  invoke<Permissions>("set_permission", { key, value });
export const recentActions = (limit?: number) =>
  invoke<Record<string, unknown>[]>("recent_actions", { limit });
export const startDaemon = () => invoke<void>("start_daemon");
export const startLogin = (appSlug: string, name: string) =>
  invoke<void>("start_login", { appSlug, name });
export const uninstall = (purge: boolean) => invoke<void>("uninstall", { purge });

export interface LoginEvent {
  event: "pending" | "approved" | "error";
  device_code?: string;
  login_url?: string;
  user_key?: string;
  email?: string;
  reason?: string;
}

export const onLoginEvent = (cb: (e: LoginEvent) => void): Promise<UnlistenFn> =>
  listen<LoginEvent>("login-event", (e) => cb(e.payload));

// First-run "setting up…" stage strings emitted during the bundled install.
// Empty string means setup finished.
export const onSetupEvent = (cb: (stage: string) => void): Promise<UnlistenFn> =>
  listen<string>("setup", (e) => cb(e.payload));
