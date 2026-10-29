import * as users from "../../shared/api/users";
import { orderStatuses } from "../../shared/api/orders";
import "../../shared/api/session";

export const profile = (id: string) => users.getUser(id);

export const statuses = () => orderStatuses;
