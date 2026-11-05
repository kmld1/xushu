const dateTimeFormatter = new Intl.DateTimeFormat("zh-CN", {
  dateStyle: "medium",
  timeStyle: "short",
  hour12: false,
});

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date);
}

export function shortId(value: string, length = 8): string {
  return value.length > length ? `${value.slice(0, length)}…` : value;
}

export function shortHead(value: string | null): string {
  return value ? value.slice(0, 8) : "—";
}
