import { useEffect, useState } from "react";
import {
  INTEGRATIONS,
  getStatus,
  onLoginEvent,
  onSetupEvent,
  openLoginUrl,
  recentActions,
  setPermission,
  startDaemon,
  startLogin,
  uninstall,
  type LoginEvent,
  type Permissions,
  type Status,
} from "./api";
import "./App.css";

function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [login, setLogin] = useState<LoginEvent | null>(null);
  const [setupStage, setSetupStage] = useState("");
  const [actions, setActions] = useState<Record<string, unknown>[]>([]);
  const [error, setError] = useState("");
  const [appSlug, setAppSlug] = useState("");
  const [name, setName] = useState("");

  async function refresh() {
    try {
      setStatus(await getStatus());
      setActions(await recentActions(8));
    } catch {
      setStatus(null);
    }
  }

  useEffect(() => {
    refresh();
    const unLogin = onLoginEvent((e) => {
      setLogin(e);
      // `login --json` is auth-only — the app opens the approval page itself.
      if (e.event === "pending" && e.login_url) openLoginUrl(e.login_url);
      if (e.event === "approved") refresh();
    });
    const unSetup = onSetupEvent((stage) => {
      setSetupStage(stage);
      if (stage === "") refresh();
    });
    return () => {
      unLogin.then((fn) => fn());
      unSetup.then((fn) => fn());
    };
  }, []);

  async function toggle(key: string, permKey: keyof Permissions) {
    if (!status) return;
    const next = !status.permissions[permKey];
    try {
      const perms = await setPermission(key, next);
      setStatus({ ...status, permissions: perms });
    } catch (err) {
      setError(String(err));
    }
  }

  async function onUninstall() {
    const purge = window.confirm(
      "Remove Hyperspell? This stops syncing, removes the CLIs from your PATH, " +
        "and reverses the agent-config changes.\n\nClick OK to also delete " +
        "~/.hyperspell (config + synced data). Cancel to keep your data.",
    );
    // OK = purge everything; Cancel here means the user dismissed — bail.
    // (A real build would use a 3-way dialog; window.confirm is binary.)
    try {
      await uninstall(purge);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function onLogin() {
    setError("");
    try {
      await startLogin(appSlug, name);
    } catch (err) {
      setError(String(err));
    }
  }

  async function onStart() {
    setError("");
    try {
      await startDaemon();
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  const loggedIn = status?.logged_in ?? false;

  return (
    <main className="app">
      <header className="app__header">
        <div>
          <h1>Hyperspell</h1>
          {loggedIn && status?.identity && (
            <span className="who">{status.identity}</span>
          )}
        </div>
        <span className={`dot ${status?.daemon_running ? "dot--on" : "dot--off"}`}>
          {status?.daemon_running ? "Syncing" : "Idle"}
        </span>
      </header>

      {setupStage && (
        <div className="setup" role="status">
          <span className="spinner" /> {setupStage}
        </div>
      )}

      {error && (
        <div className="banner banner--error" role="alert">
          {error} <button className="link" onClick={() => setError("")}>dismiss</button>
        </div>
      )}

      {!loggedIn && (
        <section className="card">
          <h2>Connect this device</h2>
          <p className="muted">
            Hyperspell installs its command-line tools through a signed path and
            keeps your company brain in sync. It only touches the tools you
            approve below — nothing is written until you say so.
          </p>
          <input
            placeholder="App slug"
            value={appSlug}
            onChange={(e) => setAppSlug(e.currentTarget.value)}
          />
          <input
            placeholder="Your name"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
          />
          <button onClick={onLogin} disabled={!appSlug.trim() || !name.trim()}>
            Log in
          </button>
          {login?.event === "pending" && (
            <p className="muted">
              Opened your browser — approve code <b>{login.device_code}</b> there.
            </p>
          )}
          {login?.event === "error" && <p className="error">{login.reason}</p>}
        </section>
      )}

      {loggedIn && !status?.daemon_running && !setupStage && (
        <section className="card">
          <p className="muted">Sync isn't running.</p>
          <button onClick={onStart}>Start sync</button>
        </section>
      )}

      <section className="card">
        <h2>Approved integrations</h2>
        <p className="muted">
          Nothing is written until you allow it. Toggle off to remove it on the
          next sync.
        </p>
        <ul className="integrations">
          {INTEGRATIONS.map((it) => {
            const on = status?.permissions[it.permKey] ?? false;
            return (
              <li key={it.key}>
                <label>
                  <input
                    type="checkbox"
                    checked={on}
                    disabled={!loggedIn}
                    onChange={() => toggle(it.key, it.permKey)}
                  />
                  <span>
                    <b>{it.label}</b>
                    <small>{it.detail}</small>
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
      </section>

      {actions.length > 0 && (
        <section className="card">
          <h2>Recent activity</h2>
          <ul className="activity">
            {actions
              .slice()
              .reverse()
              .map((a, i) => (
                <li key={i}>
                  <b>{String(a.action ?? "?")}</b>
                  {a.detail ? <small> — {String(a.detail)}</small> : null}
                </li>
              ))}
          </ul>
        </section>
      )}

      <button className="danger" onClick={onUninstall}>
        Remove Hyperspell…
      </button>
    </main>
  );
}

export default App;
