// stratum-ignore stratum/public-api -- totals is moved behind the entry next release
import { total } from "../modules/billing/totals";

export const run = () => total([1, 2]);
