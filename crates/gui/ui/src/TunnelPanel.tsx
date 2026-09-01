import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import PromptDialog from './PromptDialog';
import { PASSWORD_REQUIRED } from './adhoc';

type Tunnel = { id: string; profile: string; local: string; target: string };
type Props = { profile: string; onClose: () => void };

export default function TunnelPanel({ profile, onClose }: Props) {
  const [tunnels, setTunnels] = useState<Tunnel[]>([]);
  const [bindPort, setBindPort] = useState('8080');
  const [targetHost, setTargetHost] = useState('127.0.0.1');
  const [targetPort, setTargetPort] = useState('80');
  const [message, setMessage] = useState<string | null>(null);
  const [passwordPromptOpen, setPasswordPromptOpen] = useState(false);

  const refresh = useCallback(() => {
    invoke<Tunnel[]>('tunnel_list')
      .then(setTunnels)
      .catch((reason) => setMessage(String(reason)));
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 2000);
    return () => clearInterval(timer);
  }, [refresh]);

  const start = async (password?: string) => {
    try {
      await invoke('tunnel_start', {
        profileName: profile,
        password: password ?? null,
        bindHost: '127.0.0.1',
        bindPort: Number(bindPort),
        targetHost,
        targetPort: Number(targetPort),
      });
      setMessage('Túnel iniciado');
      if (password) {
        await invoke('save_profile_secret', { name: profile, secret: password }).catch((reason) => {
          setMessage(`Túnel iniciado, pero no se pudo guardar la contraseña: ${String(reason)}`);
        });
      }
      refresh();
    } catch (reason) {
      if (!password && String(reason).includes(PASSWORD_REQUIRED)) {
        setPasswordPromptOpen(true);
      } else {
        setMessage(String(reason));
      }
    }
  };

  const stop = async (id: string) => {
    try {
      await invoke('tunnel_stop', { id });
      refresh();
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  return (
    <div className="sftp-panel">
      <div className="sftp-header">
        <span className="terminal-dot" />
        <span className="terminal-title">Túneles · {profile}</span>
        <button className="terminal-close" aria-label={`Cerrar túneles de ${profile}`} onClick={onClose}>
          ✕
        </button>
      </div>

      <form className="tunnel-form" onSubmit={(event) => {
        event.preventDefault();
        void start();
      }}>
        <label className="field">
          <span>Puerto local</span>
          <input
            type="number"
            min={1}
            max={65535}
            value={bindPort}
            onChange={(event) => setBindPort(event.target.value)}
          />
        </label>
        <label className="field">
          <span>Host destino</span>
          <input value={targetHost} onChange={(event) => setTargetHost(event.target.value)} />
        </label>
        <label className="field">
          <span>Puerto destino</span>
          <input
            type="number"
            min={1}
            max={65535}
            value={targetPort}
            onChange={(event) => setTargetPort(event.target.value)}
          />
        </label>
        <button type="submit" className="btn primary">
          Iniciar
        </button>
      </form>

      {message && (
        <div className="pane-message">
          <span>{message}</span>
          <button className="btn ghost small" onClick={() => setMessage(null)}>
            cerrar
          </button>
        </div>
      )}

      <div className="tunnel-list">
        {tunnels.length === 0 && <p className="muted empty">No hay túneles activos.</p>}
        {tunnels.map((tunnel) => (
          <div key={tunnel.id} className="tunnel-row">
            <div className="tunnel-info">
              <span className="tunnel-local">
                <span className="live-dot" /> {tunnel.local}
              </span>
              <span className="muted">→ {tunnel.target}</span>
            </div>
            <button className="btn ghost small" onClick={() => stop(tunnel.id)}>
              Detener
            </button>
          </div>
        ))}
      </div>
      {passwordPromptOpen && (
        <PromptDialog
          title={`Contraseña para ${profile}`}
          description="No hay una contraseña guardada para este perfil. Se guardará si el túnel inicia."
          label="Contraseña"
          inputType="password"
          trimValue={false}
          confirmLabel="Iniciar"
          requireValue
          onConfirm={(password) => {
            setPasswordPromptOpen(false);
            void start(password);
          }}
          onCancel={() => setPasswordPromptOpen(false)}
        />
      )}
    </div>
  );
}
