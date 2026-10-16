import type { Invoice } from "./invoice-types";

export function render(invoice: Invoice): string {
  return `${invoice.number}: ${invoice.total}`;
}
