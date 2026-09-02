export const HOST_KEY_PREFIX = 'sshcli:host-key:';

export type HostKeyPrompt = {
  host: string;
  port: number;
  key: string;
  changed: boolean;
};

export function parseHostKeyError(message: string): HostKeyPrompt | null {
  if (!message.startsWith(HOST_KEY_PREFIX)) return null;
  try {
    const parsed = JSON.parse(message.slice(HOST_KEY_PREFIX.length)) as Partial<HostKeyPrompt>;
    if (
      typeof parsed.host !== 'string' ||
      typeof parsed.port !== 'number' ||
      typeof parsed.key !== 'string' ||
      typeof parsed.changed !== 'boolean'
    ) {
      return null;
    }
    return {
      host: parsed.host,
      port: parsed.port,
      key: parsed.key,
      changed: parsed.changed,
    };
  } catch {
    return null;
  }
}
