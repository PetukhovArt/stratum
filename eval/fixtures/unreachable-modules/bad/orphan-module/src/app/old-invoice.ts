import { total } from "./invoice";

export function legacyTotal(net: number): number {
  return total(net, 0);
}
