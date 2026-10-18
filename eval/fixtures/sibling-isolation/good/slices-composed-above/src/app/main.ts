import { billItems } from "../features/billing";
import { listProducts } from "../features/catalog";

export const run = () => billItems(listProducts());
