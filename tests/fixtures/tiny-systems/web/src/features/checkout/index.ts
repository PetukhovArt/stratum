import { orders } from "../../shared/api/orders";
import { getUser } from "../../shared/api";
import restore from "../../shared/api/session";

export async function placeOrder(userId: string) {
  const user = await getUser(userId);
  return orders.placeOrder({ user });
}
