// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import previewCatalog from "../../public/dev-catalog.json";
import { App } from "../App";
import { useAppStore } from "../stores/app";

beforeEach(() => {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      new Response(JSON.stringify(previewCatalog), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    ),
  );
  useAppStore.setState({
    catalog: [],
    catalogLoading: true,
    catalogError: null,
    selectedToolId: null,
    category: "home",
    paletteOpen: false,
    railCollapsed: false,
    recentToolIds: [],
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("M0 shell smoke", () => {
  it("loads the registry shell and opens a fuzzy-matched tool by keyboard", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Good morning." }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: /Resize image/ }),
    ).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    const palette = await screen.findByRole("dialog", {
      name: "Command palette",
    });
    expect(palette).toBeInTheDocument();

    const search = screen.getByPlaceholderText("What do you need?");
    fireEvent.change(search, {
      target: { value: "rszim" },
    });
    expect(
      screen.getByRole("option", { name: /Resize image/ }),
    ).toBeInTheDocument();

    fireEvent.keyDown(search, { key: "Enter" });
    expect(
      await screen.findByRole("heading", { name: "Resize image" }),
    ).toBeInTheDocument();
  });
});
