import { taxRate } from "./_shared/rates";

export const withTax = (net: number) => net * (1 + taxRate);
