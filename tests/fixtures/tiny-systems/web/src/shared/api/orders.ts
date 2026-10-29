import { createClient } from "@acme/rpc";
import { OrderService } from "@acme/rpc-gen";
import { rpcOptions } from "../config/rpc";

export const orders = createClient(OrderService, rpcOptions);
