import { User } from "@/entities/user";
import { Button } from "@/shared/ui/Button";

export function Cart() {
  const u: User = { id: "x", name: "Alice" };
  return <Button label={`Hello ${u.name}`} />;
}
