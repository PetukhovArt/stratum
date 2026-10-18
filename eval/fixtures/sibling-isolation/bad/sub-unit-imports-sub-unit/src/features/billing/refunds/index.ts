import { issue } from "../invoices";

export const refund = (amount: number) => issue(-amount);
