import { listProducts } from "../catalog";

export const billAll = () => listProducts().map((name) => ({ name, amount: 10 }));
