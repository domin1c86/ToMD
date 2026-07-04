import { describe, expect, it } from "vitest";
import type { UserConfig } from "vite";

import config from "../vite.config";

describe("Vite production config", () => {
  it("uses relative asset paths so Tauri can load the production bundle", () => {
    expect((config as UserConfig).base).toBe("./");
  });
});
