import { total, money } from "./billing";
import * as rates from "./rates";
import Badge from "./Badge.vue";

export const app = { Badge };

export async function run(): Promise<void> {
  const { printReceipt } = await import("./receipt");
  printReceipt(money.format(total(2, rates.TAX)));
}
