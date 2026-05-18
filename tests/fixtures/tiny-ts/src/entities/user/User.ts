import { formatId } from "@/shared/utils/format";

export interface User {
  id: string;
  name: string;
}

export const labelFor = (u: User) => formatId(u.id) + " " + u.name;
