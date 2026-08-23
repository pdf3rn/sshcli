import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FileIcon, FolderIcon } from './icons';

type Props = {
  sessionId: string;
  cwd?: string;
};

type Entry = { name: string; isDir: boolean };

const BASH_SNIPPET = `PROMPT_COMMAND='printf "\\e]7;file://%s%s\\a" "$HOSTNAME" "$PWD"'`;
const ZSH_SNIPPET = `precmd() { printf "\\e]7;file://%s%s\\a" "$HOSTNAME" "$PWD" }`;

function quote(path: string): string {
  return `'${path.replace(/'/g, `'\\''`)}'`;
}

function joinPath(dir: string, name: string): string {
  const base = dir === '/' ? '' : dir.replace(/\/$/, '');
  return `${base}/${name}`;
}

export default function RemoteExplorerPanel({ sessionId, cwd }: Props) {
  const [viewPath, setViewPath] = useState('');
  const [entries, setEntries] = useState<Entry[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState('');
  const reqId = useRef(0);

  const load = useCallback(
    async (target: string) => {
      if (!sessionId || !target) return;
      const id = ++reqId.current;
      setBusy(true);
      setError('');
      try {
        const out = await invoke<string>('ssh_exec', {
          id: sessionId,
          command: `ls -1Ap -- ${quote(target)} 2>/dev/null | head -300`,
        });
        if (reqId.current !== id) return;
        setEntries(
          out
            .split('\n')
            .filter((line) => line.length > 0)
            .map((line) =>
              line.endsWith('/')
                ? { name: line.slice(0, -1), isDir: true }
                : { name: line, isDir: false },
            ),
        );
      } catch (reason) {
        if (reqId.current !== id) return;
        setError(String(reason));
        setEntries([]);
      } finally {
        if (reqId.current === id) setBusy(false);
      }
    },
    [sessionId],
  );

  useEffect(() => {
    if (!cwd) return;
    setViewPath(cwd);
    void load(cwd);
  }, [cwd, load]);

  const copySnippet = (key: string, text: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        setCopied(key);
        window.setTimeout(() => setCopied(''), 1500);
      })
      .catch(() => undefined);
  };

  const crumbs: { label: string; path: string }[] = [];
  if (viewPath.startsWith('/')) {
    crumbs.push({ label: '/', path: '/' });
    let acc = '';
    for (const part of viewPath.split('/').filter(Boolean)) {
      acc += `/${part}`;
      crumbs.push({ label: part, path: acc });
    }
  }

  return (
    <aside className="remote-explorer" aria-label="Explorador remoto">
      <div className="explorer-head">
        <span className="explorer-title">Explorador remoto</span>
        <button
          type="button"
          className="icon-btn small"
          disabled={busy || !viewPath}
          aria-label="Refrescar carpeta"
          title="Refrescar"
          onClick={() => void load(viewPath)}
        >
          ⟳
        </button>
      </div>

      {!cwd ? (
        <div className="explorer-empty">
          <p className="muted small">
            Tu shell no reporta la carpeta actual. Añade el snippet de OSC 7 a tu rc y recarga:
          </p>
          <div className="snippet">
            <code>~/.bashrc</code>
            <button
              type="button"
              className="icon-btn small"
              aria-label={copied === 'bash' ? 'Copiado' : 'Copiar snippet para bash'}
              onClick={() => copySnippet('bash', BASH_SNIPPET)}
            >
              {copied === 'bash' ? '✓' : '⧉'}
            </button>
            <pre>{BASH_SNIPPET}</pre>
          </div>
          <div className="snippet">
            <code>~/.zshrc</code>
            <button
              type="button"
              className="icon-btn small"
              aria-label={copied === 'zsh' ? 'Copiado' : 'Copiar snippet para zsh'}
              onClick={() => copySnippet('zsh', ZSH_SNIPPET)}
            >
              {copied === 'zsh' ? '✓' : '⧉'}
            </button>
            <pre>{ZSH_SNIPPET}</pre>
          </div>
          <p className="muted small">
            Luego ejecuta <code>exec $SHELL</code> y pulsa Enter en la terminal.
          </p>
        </div>
      ) : (
        <>
          <nav className="crumbs" aria-label="Ruta actual">
            {crumbs.map((crumb, index) => (
              <span key={crumb.path} className="crumb-wrap">
                {index > 0 && <span className="crumb-sep">/</span>}
                <button
                  type="button"
                  className={`crumb ${index === crumbs.length - 1 ? 'cur' : ''}`}
                  onClick={() => {
                    setViewPath(crumb.path);
                    void load(crumb.path);
                  }}
                >
                  {crumb.label}
                </button>
              </span>
            ))}
          </nav>

          {error ? (
            <p className="explorer-error small" role="alert">
              {error}
            </p>
          ) : busy && entries.length === 0 ? (
            <div className="explorer-skeleton" aria-hidden="true">
              <span />
              <span />
              <span />
              <span />
            </div>
          ) : entries.length === 0 ? (
            <p className="muted small explorer-none">Carpeta vacía</p>
          ) : (
            <ul className="explorer-list">
              {entries.map((entry) =>
                entry.isDir ? (
                  <li key={entry.name}>
                    <button
                      type="button"
                      className="explorer-row"
                      disabled={busy}
                      onClick={() => {
                        const next = joinPath(viewPath, entry.name);
                        setViewPath(next);
                        void load(next);
                      }}
                    >
                      <span className="explorer-icon dir" aria-hidden="true">
                        <FolderIcon />
                      </span>
                      <span className="explorer-name">{entry.name}</span>
                    </button>
                  </li>
                ) : (
                  <li key={entry.name}>
                    <div className="explorer-row static">
                      <span className="explorer-icon" aria-hidden="true">
                        <FileIcon />
                      </span>
                      <span className="explorer-name">{entry.name}</span>
                    </div>
                  </li>
                ),
              )}
            </ul>
          )}
        </>
      )}
    </aside>
  );
}
