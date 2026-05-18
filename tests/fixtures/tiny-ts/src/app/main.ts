import { mountCart } from "@/features/cart";
import { http } from "~shared/api/http";

http.get("/init");
mountCart();
