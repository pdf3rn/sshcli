export type View = 'home' | 'connections' | 'session' | 'settings';

export type Profile = {
  name: string;
  host: string;
  port: number;
  username: string;
  identity_file: string | null;
  authentication: 'None' | 'Password' | 'PrivateKey';
  accept_unknown_host_key: boolean;
};

export type Tab =
  | { kind: 'terminal'; id: string; profile: string; connected: boolean }
  | { kind: 'sftp'; id: string; profile: string }
  | { kind: 'tunnels'; id: string; profile: string };
