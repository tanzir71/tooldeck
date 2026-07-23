import { describe, expect, it } from "vitest";

import type { ToolMeta } from "./registry";
import { byCategory, scoreTool, searchCatalog } from "./registry";

const tools: ToolMeta[] = [
  {
    id: "image-resize",
    name: "Resize image",
    summary: "",
    category: "images",
    tier: "core",
    icon: "image",
    keywords: ["scale", "photo"],
    form: null,
    destructive: false,
    longRunning: false,
  },
  {
    id: "pdf-merge",
    name: "Merge PDFs",
    summary: "",
    category: "pdf",
    tier: "core",
    icon: "files",
    keywords: ["combine", "join"],
    form: null,
    destructive: false,
    longRunning: true,
  },
];

describe("tool catalog helpers", () => {
  it("prioritizes direct name matches", () => {
    expect(searchCatalog(tools, "resize")[0]?.id).toBe("image-resize");
    expect(scoreTool(tools[1], "combine")).toBeGreaterThan(0);
  });

  it("supports subsequence matching", () => {
    expect(searchCatalog(tools, "rszim")[0]?.id).toBe("image-resize");
  });

  it("groups tools for registry-generated category views", () => {
    expect(byCategory(tools).images).toHaveLength(1);
    expect(byCategory(tools).pdf?.[0]?.id).toBe("pdf-merge");
  });
});
