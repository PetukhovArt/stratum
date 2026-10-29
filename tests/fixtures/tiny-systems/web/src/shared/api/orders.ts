import { createClient } from "@acme/rpc";
import { OrderService } from "@acme/rpc-gen";

export const orders = createClient(OrderService, { baseUrl: "/rpc" });
