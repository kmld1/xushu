export type AppInfo = {
  name: string;
  version: string;
  tauriVersion: string;
  phase: string;
  platform: string;
};

export type DatabaseHealth = {
  ok: boolean;
  integrityOk: boolean;
  foreignKeysEnabled: boolean;
  foreignKeyViolations: boolean;
  journalMode: string;
  schemaVersion: number;
  recordCount: number;
  databaseFile: string;
};
