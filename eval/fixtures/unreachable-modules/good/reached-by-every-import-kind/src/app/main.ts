import "./polyfills";
import type { Invoice } from "./invoice-types";

export async function print(invoice: Invoice): Promise<void> {
  const { render } = await import("./renderer");
  console.log(render(invoice));
}
