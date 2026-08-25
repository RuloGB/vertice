import { invoke } from "@tauri-apps/api/core";

/**
 * Invoke the `log_file_path` command: the absolute path of the application
 * log file, returned as a plain string so it can be rendered as selectable
 * text. Performs no I/O beyond a path join on the Rust side — it never
 * creates, opens, or modifies the log file or its directory.
 */
export function fetchLogFilePath(): Promise<string> {
  return invoke<string>("log_file_path");
}
