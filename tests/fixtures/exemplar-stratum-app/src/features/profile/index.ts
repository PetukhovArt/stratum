import { UserModel } from "@/entities/user";
import { fmt } from "@/shared/utils";
export const showProfile = (u: UserModel) => fmt(u.name("y"));
