export function formatMoney(value: number): string {
  return value.toFixed(2);
}

export function formatPercent(value: number): string {
  return `${value * 100}%`;
}
