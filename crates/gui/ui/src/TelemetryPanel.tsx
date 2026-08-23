import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type TelemetrySample = {
  cpuPercent: number;
  memUsedMb: number;
  memTotalMb: number;
  diskUsedGb: number;
  diskTotalGb: number;
  rxKbps: number;
  txKbps: number;
};

type Props = { profile: string };

const POLL_MS = 3000;

function fmtRate(kbps: number): string {
  if (kbps >= 1024) return `${(kbps / 1024).toFixed(1)} MB/s`;
  return `${kbps.toFixed(1)} KB/s`;
}

function Meter({
  label,
  value,
  max,
  text,
}: {
  label: string;
  value: number;
  max: number;
  text: string;
}) {
  const percent = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  const clamped = Math.round(percent);
  return (
    <div className="telemetry-meter">
      <div className="meter-head">
        <span className="meter-label">{label}</span>
        <span className="meter-value">{text}</span>
      </div>
      <div
        className="meter-track"
        role="meter"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={clamped}
        aria-label={`${label}: ${text}`}
      >
        <div className="meter-fill" style={{ width: `${percent}%` }} />
      </div>
    </div>
  );
}

export default function TelemetryPanel({ profile }: Props) {
  const [sample, setSample] = useState<TelemetrySample | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = () => {
      invoke<TelemetrySample>('telemetry_sample', { profileName: profile })
        .then((next) => {
          if (!cancelled) {
            setSample(next);
            setError(null);
          }
        })
        .catch((reason) => {
          if (!cancelled) setError(String(reason));
        });
    };
    tick();
    const interval = setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
      void invoke('telemetry_disconnect', { profileName: profile }).catch(
        () => undefined,
      );
    };
  }, [profile]);

  return (
    <aside className="telemetry-panel" aria-label={`Telemetría de ${profile}`}>
      <h3 className="panel-label">Host Telemetry</h3>
      <p className="telemetry-profile">{profile}</p>
      {error ? (
        <p className="muted small telemetry-error" role="status">
          No disponible
        </p>
      ) : sample ? (
        <div className="telemetry-body">
          <Meter
            label="CPU"
            value={sample.cpuPercent}
            max={100}
            text={`${sample.cpuPercent.toFixed(0)}%`}
          />
          <Meter
            label="MEM"
            value={sample.memUsedMb}
            max={sample.memTotalMb}
            text={`${(sample.memUsedMb / 1024).toFixed(1)} / ${(sample.memTotalMb / 1024).toFixed(1)} GB`}
          />
          <Meter
            label="Storage"
            value={sample.diskUsedGb}
            max={sample.diskTotalGb}
            text={`${sample.diskUsedGb.toFixed(1)} / ${sample.diskTotalGb.toFixed(1)} GB`}
          />
          <div className="telemetry-net">
            <div className="net-row">
              <span className="meter-label">Network RX</span>
              <span className="meter-value">{fmtRate(sample.rxKbps)}</span>
            </div>
            <div className="net-row">
              <span className="meter-label">TX</span>
              <span className="meter-value">{fmtRate(sample.txKbps)}</span>
            </div>
          </div>
        </div>
      ) : (
        <p className="muted small" aria-live="polite">
          Muestreando…
        </p>
      )}
    </aside>
  );
}
