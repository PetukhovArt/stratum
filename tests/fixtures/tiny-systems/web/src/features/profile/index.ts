import * as users from "../../shared/api/users";
import "../../shared/api/session";

export const profile = (id: string) => users.getUser(id);
