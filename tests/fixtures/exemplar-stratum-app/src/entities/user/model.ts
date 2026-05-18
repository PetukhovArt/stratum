import { fmt } from "@/shared/utils";
export class UserModel {
  name(s: string) {
    return fmt(s);
  }
}
