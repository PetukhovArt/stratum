import { invoke } from "@tauri-apps/api/core";

export async function saveFile(path: string, data: number[]) {
  return invoke("save_file", { path, data });
}

export const greet = (name: string) => invoke<string>("greet", { name });

export const run = (command: string) => invoke(command);
