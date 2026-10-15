import { boom } from "./boom";
import { applyCoupon } from "./coupon";
import { cartItems } from "./model/store";

declare function inject(key: string): unknown;

boom();
cartItems.push(applyCoupon("x"));
inject("cart");
