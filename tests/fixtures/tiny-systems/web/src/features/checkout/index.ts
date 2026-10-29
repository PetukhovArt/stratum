import { orders } from "../../shared/api/orders";
import { getUser } from "../../shared/api/users";

export async function placeOrder(userId: string) {
  const user = await getUser(userId);
  return orders.placeOrder({ user });
}
