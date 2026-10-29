import { createClient } from "@acme/rpc";
import { OrderService } from "@acme/rpc-gen";
import { rpcOptions } from "../config/rpc";

export const createOrdersApi = (options: typeof rpcOptions) => {
  const client = createClient(OrderService, options);
  return { placeOrder: (user: string) => client.placeOrder({ user }) };
};

export const ordersApi = createOrdersApi(rpcOptions);

export const orderStatuses = ["open", "placed"];

export const orders = createClient(OrderService, rpcOptions);
