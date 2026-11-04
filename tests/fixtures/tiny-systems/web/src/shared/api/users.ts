import { createClient } from "@acme/rpc";
import { UserService } from "@acme/rpc-gen";

const users = createClient(UserService, { baseUrl: "/rpc" });

export function getUser(id: string) {
  return users.getUser({ id });
}

export const watchUsers = () => users.watchUsers({});

export const forget = () => users.forgetUser({});
