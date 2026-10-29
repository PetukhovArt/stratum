import { invoke } from "@tauri-apps/api/core";

invoke("greet", { name: "session" });

export default function restore() {
  return invoke("save_file", { path: "session", data: [] });
}
