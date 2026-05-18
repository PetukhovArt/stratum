import { UserModel } from "@/entities/user";
import { fmt } from "@/shared/utils";
export const startCheckout = (u: UserModel) => fmt(u.name("x"));
