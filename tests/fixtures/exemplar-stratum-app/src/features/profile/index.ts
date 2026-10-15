import { UserModel } from "@/entities/user";
import { fmt } from "@/shared/utils";
import { token } from "@/shared/_shared/token";
import { startCheckout } from "@/features/checkout";
export const showProfile = (u: UserModel) => fmt(u.name("y")) + token + startCheckout(u);
