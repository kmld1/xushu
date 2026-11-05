export type ProbeRecord = {
  id: string;
  label: string;
  content: string;
  createdAt: string;
};

export type CreateProbeInput = {
  label: string;
  content: string;
};
