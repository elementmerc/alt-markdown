import { describe, expect, it } from "vitest";

import { VERSION } from "./index";

describe("@altmd/runtime", () => {
  it("exposes a version string", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
