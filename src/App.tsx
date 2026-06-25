import { useEffect, useState } from "react";
import {
  INTEGRATIONS,
  getStatus,
  onLoginEvent,
  setPermission,
  startLogin,
  type LoginEvent,
  type Permissions,
  type Status,
} from "./api";
import "./App.css";

function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [login, setLogin] = useState<LoginEvent | null>(null);
  const [appSlug, setAppSlug] = useState("");
  const [name, setName] = useState("");

  async function refresh() {
    try {
      setStatus(await getStatus());
    } catch {
      setStatus(null);
    }
  }

  useEffect(() => {
    refresh();
    const unlisten = onLoginEvent((e) => {
      setLogin(e);
      if (e.event === "approved") refresh();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function toggle(key: string, permKey: keyof Permissions) {
    if (!status) return;
    const next = !status.permissions[permKey];
    try {
      const perms = await setPermission(key, next);
      setStatus({ ...status, permissions: perms });
    } catch (err) {
      console.error(err);
    }
  }

  const loggedIn = !!status; // TODO: surface real auth state from config

  return (
    <main className="app">
      <header className="app__header">
        <h1>Hyperspell</h1>
        <span className={`dot ${status?.daemon_running ? "dot--on" : "dot--off"}`}>
          {status?.daemon_running ? "Syncing" : "Idle"}
        </span>
      </header>

      {!loggedIn && (
        <section className="card">
          <h2>Connect this device</h2>
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
          <button onClick={() => startLogin(appSlug, name)}>Log in</button>
          {login?.event === "pending" && (
            <p className="muted">
              Code <b>{login.device_code}</b> — approve in your browser.
            </p>
          )}
          {login?.event === "error" && <p className="error">{login.reason}</p>}
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
    </main>
  );
}

export default App;
